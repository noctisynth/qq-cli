#![cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[path = "common/managed.rs"]
mod managed;
use qq_cli::core::domain::{Conversation, Scope};
use qq_cli::storage::credentials::{Credentials, SystemCredentials};
use qq_cli::storage::references::References;
use qq_cli::storage::state::{Installation, Instance, Store};
use serde_json::{Value, json};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Cleanup(SystemCredentials);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.0.delete("synthetic-media");
        let _ = self.0.delete("reference-signing-key");
    }
}

fn run(config: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .env_remove("QQ_CLI_CONFIG")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(8);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Synthetic media CLI did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains("synthetic-cli-token"));
    (
        output.status.code().unwrap(),
        serde_json::from_str(text).unwrap(),
    )
}

#[test]
fn media_cli_infers_scoped_file_source_and_publishes_only_verified_bytes() {
    if managed::child() {
        return;
    }
    for private in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let vault = Cleanup(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-media", "synthetic-cli-token")
            .unwrap();
        let mut raw = json!({"message_id":"40000","real_seq":"100","time":1700000000,"self_id":"10001","user_id":"10002","message_type":if private {"private"}else{"group"},"message":[{"type":"file","data":{"file":"sample.bin","file_id":"opaque-file-uuid","file_size":"3"}}]});
        raw[if private { "target_id" } else { "group_id" }] =
            json!(if private { "10001" } else { "20001" });
        let original = raw.clone();
        let (endpoint, server) = http::serve(17, move |action, payload| {
            let data = match action {
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_msg" => {
                    assert_eq!(payload["message_id"], "40000");
                    original.clone()
                }
                "get_file" => {
                    assert_eq!(payload, &json!({"file_id":"opaque-file-uuid"}));
                    json!({"base64":"YWJj","file_size":"3","file":"untrusted-path","url":"https://invalid.invalid/ignored"})
                }
                _ => panic!("Unexpected synthetic media action"),
            };
            serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
        });
        let mut transaction = store
            .transaction(Instant::now() + Duration::from_secs(3))
            .unwrap();
        transaction.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("unused-installation"),
        });
        let instance = Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-media".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        };
        transaction.registry.instances.push(instance.clone());
        let mut other = instance;
        other.id = "other-instance".into();
        other.credential_key = "missing-unrelated-key".into();
        transaction.registry.instances.push(other);
        transaction.commit().unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &vault.0,
            "instance-example",
            "media_cli_infers_scoped_file_source_and_publishes_only_verified_bytes",
        );
        let refs =
            References::open(&store, &vault.0, Instant::now() + Duration::from_secs(3)).unwrap();
        let message = qq_cli::messaging::message::normalize(
            &raw,
            &Scope {
                instance_id: "instance-example".into(),
                account: "10001".into(),
            },
            "run-example",
            &if private {
                Conversation::Private("10002".into())
            } else {
                Conversation::Group("20001".into())
            },
            &refs,
        )
        .unwrap();
        let media_ref = message.value["segments"][0]["mediaRef"].as_str().unwrap();
        let config = temp.path().join("config.json");
        std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
        let output = temp.path().join("download.bin");
        let args = [
            "media",
            "download",
            media_ref,
            "--output",
            output.to_str().unwrap(),
        ];
        let mut conflict = args.to_vec();
        conflict.extend(["--instance", "other-instance"]);
        let (code, result) = run(&config, &conflict);
        assert_eq!(code, 2);
        assert_eq!(result["error"]["code"], "INSTANCE_CONFLICT");
        assert!(!output.exists());
        let (code, result) = run(&config, &args);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["meta"]["scope"]["account"], "10001");
        assert_eq!(result["data"]["sizeBytes"], 3);
        assert_eq!(std::fs::read(&output).unwrap(), b"abc");
        let descriptor = qq_cli::cli::registry::get("media.download").unwrap();
        assert!(
            jsonschema::validator_for(&descriptor["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{result}"
        );
        server.join().unwrap();
        let (code, result) = run(&config, &args);
        assert_eq!(code, 2);
        assert_eq!(result["error"]["code"], "MEDIA_OUTPUT_INVALID");
        assert_eq!(std::fs::read(output).unwrap(), b"abc");
    }
}

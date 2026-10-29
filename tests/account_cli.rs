#![cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[path = "common/managed.rs"]
mod managed;
use base64::{Engine, engine::general_purpose::STANDARD};
use qq_cli::storage::{
    credentials::{Credentials, SystemCredentials},
    state::{Installation, Instance, Store},
};
use serde_json::{Value, json};
use std::{
    process::Command,
    time::{Duration, Instant},
};
fn run(config: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .args(["--config", config.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
#[test]
fn account_tasks_send_one_typed_action_and_share_keys_with_explicit_api() {
    if managed::child() {
        return;
    }
    for kind in [
        "profile",
        "signature",
        "avatar",
        "presence",
        "status-text",
        "rejected",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-account");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
        credentials
            .0
            .set("synthetic-account", "synthetic-cli-token")
            .unwrap();
        let (endpoint, server) = http::serve(7, move |action, payload| {
            let data = match action {
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "set_qq_profile" => {
                    assert!(matches!(kind, "profile" | "rejected"));
                    assert_eq!(payload["nickname"], "Synthetic");
                    assert_eq!(payload["personal_note"], "");
                    assert_eq!(payload["sex"].as_f64(), Some(1.0));
                    json!({"result":if kind=="rejected"{37}else{0},"errMsg":"synthetic-private-error"})
                }
                "set_self_longnick" => {
                    assert_eq!(kind, "signature");
                    assert_eq!(payload, &json!({"longNick":""}));
                    json!({"result":0})
                }
                "set_qq_avatar" => {
                    assert_eq!(kind, "avatar");
                    assert_eq!(
                        payload["file"],
                        format!("base64://{}", STANDARD.encode(b"synthetic-avatar-bytes"))
                    );
                    Value::Null
                }
                "set_online_status" => {
                    assert_eq!(kind, "presence");
                    assert_eq!(payload["status"].as_f64(), Some(40.0));
                    assert_eq!(payload["battery_status"].as_f64(), Some(88.0));
                    Value::Null
                }
                "set_diy_online_status" => {
                    assert_eq!(kind, "status-text");
                    assert_eq!(payload["wording"], "Synthetic");
                    assert_eq!(payload["face_id"], "123");
                    json!("")
                }
                _ => panic!("Unexpected synthetic action {action}"),
            };
            serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
        });
        let mut tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        tx.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("unused-installation"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-account".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &credentials.0,
            "instance-example",
            "account_tasks_send_one_typed_action_and_share_keys_with_explicit_api",
        );
        let server = server.start();
        let config = temp.path().join("config.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&json!({"stateDir":store.root()})).unwrap(),
        )
        .unwrap();
        let avatar = temp.path().join("avatar.png");
        std::fs::write(&avatar, b"synthetic-avatar-bytes").unwrap();
        let (leaf, mut args) = match kind {
            "profile" | "rejected" => (
                "account.set",
                vec![
                    "account",
                    "set",
                    "--nickname",
                    "Synthetic",
                    "--signature=",
                    "--sex",
                    "male",
                ],
            ),
            "signature" => ("account.set", vec!["account", "set", "--signature="]),
            "avatar" => (
                "account.avatar",
                vec!["account", "avatar", avatar.to_str().unwrap()],
            ),
            "presence" => (
                "account.presence",
                vec![
                    "account",
                    "presence",
                    "--status",
                    "40",
                    "--battery-status",
                    "88",
                ],
            ),
            _ => (
                "account.status-text",
                vec!["account", "status-text", "Synthetic", "--face-id", "123"],
            ),
        };
        args.extend(["--request-id", "account-example"]);
        let (code, result) = run(&config, &args);
        assert_eq!(code, if kind == "rejected" { 4 } else { 0 }, "{result}");
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("synthetic-private-error")
        );
        let schema = qq_cli::cli::registry::get(leaf).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
        server.join().unwrap();
        let (code, cached) = run(&config, &args);
        assert_eq!(code, if kind == "rejected" { 4 } else { 0 });
        if kind == "rejected" {
            assert_eq!(cached["error"]["outcome"], "rejected");
            assert_eq!(cached["error"]["requestId"], "account-example");
        } else {
            assert_eq!(cached["data"]["deduplicated"], true);
        }
        if kind == "signature" {
            let input = temp.path().join("raw.json");
            std::fs::write(&input, br#"{"longNick":""}"#).unwrap();
            let (code, raw) = run(
                &config,
                &[
                    "api",
                    "call",
                    "set_self_longnick",
                    "--params-file",
                    input.to_str().unwrap(),
                    "--request-id",
                    "account-example",
                ],
            );
            assert_eq!(code, 0);
            assert_eq!(raw["data"]["deduplicated"], true);
            assert_eq!(raw["data"]["resultAvailable"], false);
        }
    }
}

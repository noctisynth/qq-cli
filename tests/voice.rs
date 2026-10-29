use serde_json::{Value, json};
use std::process::Command;
fn run(args: &[&str], config: &std::path::Path) -> (i32, Value) {
    let o = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(o.stderr.is_empty());
    (
        o.status.code().unwrap(),
        serde_json::from_slice(&o.stdout).unwrap(),
    )
}
#[test]
fn voice_contracts_require_explicit_intent_and_limit_formats() {
    let temp = tempfile::tempdir().unwrap();
    let absent = temp.path().join("missing.json");
    assert_eq!(
        run(&["message", "transcribe", "synthetic-ref"], &absent).0,
        2
    );
    assert!(!temp.path().join("state").exists());
    let description = qq_cli::cli::registry::get("message.transcribe").unwrap();
    assert_eq!(description["requestId"], "required unless dry-run");
    let formats = qq_cli::cli::registry::get("media.download").unwrap();
    assert_eq!(
        formats["inputSchema"]["properties"]["format"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    for (name, effect) in [("get_record", "read"), ("fetch_ptt_text", "write")] {
        let d = qq_cli::transport::catalog::describe(&name.parse().unwrap());
        assert_eq!(serde_json::to_value(d).unwrap()["effect"], effect);
    }
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn transcription_is_source_bound_once_only_and_never_persists_text() {
    use qq_cli::{
        core::domain::Scope,
        messaging::message::Locator,
        storage::{
            credentials::{Credentials, SystemCredentials},
            references::References,
            state::{Installation, Instance, Store},
        },
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in [
        "success",
        "empty",
        "failed",
        "non-record",
        "stale-run",
        "source-denied",
        "read-denied",
        "send-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-voice");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-voice", "synthetic-cli-token")
            .unwrap();
        let count = match mode {
            "success" | "empty" | "failed" => 14,
            "non-record" => 10,
            "stale-run" => 3,
            _ => 0,
        };
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let (endpoint, server) = http::serve(count, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_msg" => {
                    json!({"message_id":42,"message_type":"group","group_id":20001,"user_id":10002,"time":100,"real_seq":7,"message":[{"type":if mode=="non-record"{"text"}else{"record"},"data":{"file":"voice.silk","file_size":"3","text":"synthetic-original"}}]})
                }
                "fetch_ptt_text" => {
                    assert_eq!(payload, &json!({"message_id":"42"}));
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    json!({"text":if mode=="empty"{""}else{"synthetic-private-transcript"},"secret":"synthetic-native-secret"})
                }
                _ => panic!("unexpected synthetic action"),
            };
            serde_json::to_vec(&json!({"status":if action=="fetch_ptt_text"&&mode=="failed"{"failed"}else{"ok"},"retcode":if action=="fetch_ptt_text"&&mode=="failed"{1200}else{0},"data":data})).unwrap()
        });
        let mut tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        tx.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("unused"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-voice".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let refs =
            References::open(&store, &creds.0, Instant::now() + Duration::from_secs(5)).unwrap();
        let token = refs
            .issue(
                "m1",
                &Locator {
                    scope: Scope {
                        instance_id: "instance-example".into(),
                        account: "10001".into(),
                    },
                    run_id: if mode == "stale-run" {
                        "old-run"
                    } else {
                        "run-example"
                    }
                    .into(),
                    conversation: "group:20001".parse().unwrap(),
                    backend_id: "42".into(),
                    time_seconds: Some(100),
                    sequence: Some("7".into()),
                    sender_id: Some("10002".into()),
                },
            )
            .unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "transcription_is_source_bound_once_only_and_never_persists_text",
        );
        let config = temp.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"read":mode!="read-denied","send":mode!="send-denied","targets":[if mode=="source-denied"{"group:20002"}else{"group:20001"}]}})).unwrap()).unwrap();
        if mode == "success" {
            let (code, preview) = run(&["message", "transcribe", &token, "--dry-run"], &config);
            assert_eq!(code, 0, "{preview}");
            assert_eq!(preview["data"]["identityVerified"], false);
        }
        let server = server.start();
        let args = [
            "message",
            "transcribe",
            &token,
            "--request-id",
            "voice-example",
        ];
        let (code, result) = run(&args, &config);
        let expected = match mode {
            "success" => 0,
            "empty" | "failed" => 5,
            _ => 4,
        };
        assert_eq!(code, expected, "{mode}: {result}");
        let descriptor = qq_cli::cli::registry::get("message.transcribe").unwrap();
        assert!(
            jsonschema::validator_for(&descriptor["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{result}"
        );
        server.join().unwrap();
        let launched = matches!(mode, "success" | "empty" | "failed");
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(launched)
        );
        if launched {
            let (code, cached) = run(&args, &config);
            assert_eq!(code, expected, "{cached}");
            if mode == "success" {
                assert_eq!(result["data"]["text"], "synthetic-private-transcript");
                assert_eq!(cached["data"]["textAvailable"], false);
                assert!(cached["data"].get("text").is_none());
            }
            let (_, saved) = run(
                &[
                    "request",
                    "get",
                    "voice-example",
                    "--instance",
                    "instance-example",
                ],
                &config,
            );
            assert_eq!(saved["data"]["resultAvailable"], false);
        }
        fn check(path: &std::path::Path) {
            for e in std::fs::read_dir(path).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    check(&p);
                } else {
                    let s = String::from_utf8_lossy(&std::fs::read(p).unwrap()).into_owned();
                    assert!(!s.contains("synthetic-private-transcript"));
                    assert!(!s.contains("synthetic-native-secret"));
                }
            }
        }
        check(store.root());
    }
}

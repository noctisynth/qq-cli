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
fn read_marking_plans_are_offline_and_scope_is_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.json");
    for (args, all) in [
        (
            vec![
                "message",
                "mark-read",
                "--conversation",
                "group:20001",
                "--dry-run",
            ],
            false,
        ),
        (vec!["message", "mark-read", "--all", "--dry-run"], true),
    ] {
        let (code, result) = run(&args, &missing);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["data"]["allConversations"], all);
        assert_eq!(result["data"]["identityVerified"], false);
        let schema = qq_cli::cli::registry::get("message.mark-read").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
    }
    for args in [
        vec!["message", "mark-read", "--dry-run"],
        vec![
            "message",
            "mark-read",
            "--conversation",
            "group:20001",
            "--all",
            "--dry-run",
        ],
        vec!["message", "mark-read", "--conversation", "group:20001"],
        vec!["message", "recall", "synthetic-ref"],
    ] {
        assert_eq!(run(&args, &missing).0, 2);
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
fn owned_message_actions_validate_locators_inside_the_intent_and_never_replay() {
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
        "recall",
        "reference-read",
        "private-read",
        "group-read",
        "all-read",
        "stale-run",
        "mismatch",
        "source-denied",
        "read-denied",
        "all-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-message-actions");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-message-actions", "synthetic-cli-token")
            .unwrap();
        let with_ref = matches!(
            mode,
            "recall"
                | "reference-read"
                | "stale-run"
                | "mismatch"
                | "source-denied"
                | "read-denied"
        );
        let expected = match mode {
            "recall" | "reference-read" => 14,
            "mismatch" => 10,
            "stale-run" | "read-denied" => 3,
            "source-denied" | "all-denied" => 0,
            _ => 7,
        };
        let business = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_business = business.clone();
        let (endpoint, server) = http::serve(expected, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_msg" => {
                    assert_eq!(payload, &json!({"message_id":"42"}));
                    json!({"message_id":42,"message_type":"group","group_id":20001,"user_id":10002,"time":if mode=="mismatch"{101}else{100},"real_seq":7,"sender":{"user_id":10002,"nickname":"Synthetic"},"message":[{"type":"text","data":{"text":"synthetic-source-body"}}]})
                }
                _ => {
                    server_business.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let wanted = match mode {
                        "recall" => "delete_msg",
                        "reference-read" => "mark_msg_as_read",
                        "private-read" => "mark_private_msg_as_read",
                        "group-read" => "mark_group_msg_as_read",
                        "all-read" => "_mark_all_as_read",
                        _ => panic!("forbidden business request"),
                    };
                    assert_eq!(action, wanted);
                    let wanted = match mode {
                        "recall" | "reference-read" => json!({"message_id":"42"}),
                        "private-read" => json!({"user_id":"10002"}),
                        "group-read" => json!({"group_id":"20001"}),
                        _ => json!({}),
                    };
                    assert_eq!(payload, &wanted);
                    Value::Null
                }
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
            directory: temp.path().join("unused"),
        });
        let instance = Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-message-actions".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        };
        tx.registry.instances.push(instance.clone());
        if with_ref {
            let mut other = instance;
            other.id = "unrelated-instance".into();
            other.credential_key = "unread-unrelated-credential".into();
            other.endpoint = "http://127.0.0.1:1/".into();
            tx.registry.instances.push(other);
        }
        tx.commit().unwrap();
        let references =
            References::open(&store, &creds.0, Instant::now() + Duration::from_secs(5)).unwrap();
        let locator = Locator {
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
        };
        let token = references.issue("m1", &locator).unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "owned_message_actions_validate_locators_inside_the_intent_and_never_replay",
        );
        let server = server.start();
        let config = temp.path().join("config.json");
        let targets = match mode {
            "all-read" => vec![],
            "private-read" => vec!["private:10002"],
            "source-denied" => vec!["group:20002"],
            _ => vec!["group:20001"],
        };
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":targets,"read":mode!="read-denied"}})).unwrap()).unwrap();
        let (path, mut args) = match mode {
            "private-read" => (
                "message.mark-read",
                vec!["message", "mark-read", "--conversation", "private:10002"],
            ),
            "group-read" => (
                "message.mark-read",
                vec!["message", "mark-read", "--conversation", "group:20001"],
            ),
            "all-read" | "all-denied" => {
                ("message.mark-read", vec!["message", "mark-read", "--all"])
            }
            "reference-read" => (
                "message.mark-read",
                vec!["message", "mark-read", "--message-ref", token.as_str()],
            ),
            _ => ("message.recall", vec!["message", "recall", token.as_str()]),
        };
        if !matches!(mode, "stale-run") {
            let mut preview = args.clone();
            preview.push("--dry-run");
            let (code, result) = run(&preview, &config);
            assert_eq!(code, 0, "{mode}: {result}");
            assert_eq!(result["data"]["identityVerified"], false);
        }
        args.extend(["--request-id", "message-action-example"]);
        let (code, result) = run(&args, &config);
        let success = matches!(
            mode,
            "recall" | "reference-read" | "private-read" | "group-read" | "all-read"
        );
        assert_eq!(code, if success { 0 } else { 4 }, "{mode}: {result}");
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{mode}: {result}"
        );
        server.join().unwrap();
        assert_eq!(
            business.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(success)
        );
        if success {
            let (code, cached) = run(&args, &config);
            assert_eq!(code, 0, "{cached}");
            assert_eq!(cached["data"]["deduplicated"], true);
            if with_ref {
                std::fs::write(
                    &config,
                    serde_json::to_vec(&json!({"stateDir":store.root()})).unwrap(),
                )
                .unwrap();
                let raw = temp.path().join("raw.json");
                std::fs::write(&raw, br#"{"message_id":"42"}"#).unwrap();
                let action = if mode == "recall" {
                    "delete_msg"
                } else {
                    "mark_msg_as_read"
                };
                let (code, result) = run(
                    &[
                        "api",
                        "call",
                        action,
                        "--instance",
                        "instance-example",
                        "--params-file",
                        raw.to_str().unwrap(),
                        "--request-id",
                        "message-action-example",
                    ],
                    &config,
                );
                assert_eq!(code, 4, "{result}");
                assert_eq!(result["error"]["code"], "REQUEST_CONFLICT");
            }
        }
        if with_ref {
            let (code, result) = run(
                &[
                    "message",
                    "recall",
                    &token,
                    "--request-id",
                    "conflict-example",
                    "--instance",
                    "unrelated-instance",
                ],
                &config,
            );
            assert_eq!(code, 2);
            assert_eq!(result["error"]["code"], "INSTANCE_CONFLICT");
        }
        fn scan(path: &std::path::Path) {
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    scan(&entry.path());
                } else if let Ok(s) = std::fs::read_to_string(entry.path()) {
                    assert!(!s.contains("synthetic-source-body"));
                }
            }
        }
        scan(store.root());
    }
}

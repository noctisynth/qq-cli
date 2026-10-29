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
fn group_message_tasks_need_explicit_state_and_sign_preview_is_offline() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("missing.json");
    for args in [
        vec!["message", "essence", "synthetic-ref", "--enabled", "true"],
        vec!["message", "todo", "synthetic-ref", "--state", "open"],
        vec![
            "message",
            "todo",
            "synthetic-ref",
            "--state",
            "anything",
            "--dry-run",
        ],
        vec!["group", "sign", "20001"],
    ] {
        assert_eq!(run(&args, &config).0, 2);
    }
    let (c, v) = run(&["group", "sign", "20001", "--dry-run"], &config);
    assert_eq!(c, 0, "{v}");
    assert_eq!(v["data"]["identityVerified"], false);
    assert_eq!(
        run(&["group", "sign", "9007199254740993", "--dry-run"], &config).0,
        2
    );
    assert!(!temp.path().join("state").exists());
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_group_content_binds_message_identity_and_preserves_first_page_uncertainty() {
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
        "essence-on",
        "essence-off",
        "todo-open",
        "todo-complete",
        "todo-cancel",
        "todo-long",
        "sign",
        "signed",
        "signed-bad-id",
        "stale",
        "private",
        "no-seq",
        "long-essence",
        "read-denied",
        "todo-read-denied",
        "target-denied",
        "signed-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-group-content");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-group-content", "synthetic-cli-token")
            .unwrap();
        let count = match mode {
            "sign" | "signed" | "signed-bad-id" => 7,
            "stale" => 10,
            "todo-read-denied" => 3,
            "private" | "no-seq" | "long-essence" | "read-denied" | "target-denied"
            | "signed-denied" => 0,
            _ => 14,
        };
        let id = if matches!(mode, "todo-long" | "long-essence") {
            "4000000000"
        } else {
            "42"
        };
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let (endpoint, server) = http::serve(count, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_msg" => {
                    json!({"message_id":id,"message_type":"group","group_id":20001,"user_id":10002,"time":100,"real_seq":if mode=="stale"{8}else{7},"message":[{"type":"text","data":{"text":"synthetic-private-content"}}]})
                }
                _ => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let wanted = match mode {
                        "essence-off" => "delete_essence_msg",
                        "todo-open" | "todo-long" => "set_group_todo",
                        "todo-complete" => "complete_group_todo",
                        "todo-cancel" => "cancel_group_todo",
                        "sign" => "set_group_sign",
                        "signed" | "signed-bad-id" => "get_group_signed_list",
                        _ => "set_essence_msg",
                    };
                    assert_eq!(action, wanted);
                    let wanted = if mode.starts_with("todo") {
                        json!({"group_id":"20001","message_seq":"7"})
                    } else if matches!(mode, "sign" | "signed" | "signed-bad-id") {
                        json!({"group_id":"20001"})
                    } else {
                        json!({"message_id":"42"})
                    };
                    assert_eq!(payload, &wanted);
                    if mode.starts_with("signed") {
                        json!([{"user_id":if mode=="signed-bad-id"{9_007_199_254_740_993_u64}else{10002},"nick":"Synthetic","time":1700000000000_u64,"rank":1.5,"secret":"synthetic-native-secret"}])
                    } else {
                        Value::Null
                    }
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
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-group-content".into(),
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
                    run_id: "run-example".into(),
                    conversation: if mode == "private" {
                        "private:10002"
                    } else {
                        "group:20001"
                    }
                    .parse()
                    .unwrap(),
                    backend_id: id.into(),
                    time_seconds: Some(100),
                    sequence: if mode == "no-seq" {
                        None
                    } else {
                        Some("7".into())
                    },
                    sender_id: Some("10002".into()),
                },
            )
            .unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "managed_group_content_binds_message_identity_and_preserves_first_page_uncertainty",
        );
        let config = temp.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[if mode.ends_with("denied")&&!mode.ends_with("read-denied"){"group:20002"}else{"group:20001"}],"read":!mode.ends_with("read-denied")}})).unwrap()).unwrap();
        let (path, mut args) = if mode == "sign" {
            ("group.sign", vec!["group", "sign", "20001"])
        } else if mode.starts_with("signed") {
            (
                "group.signed",
                vec!["group", "signed", "20001", "--limit", "1"],
            )
        } else if mode.starts_with("todo") || mode == "no-seq" {
            (
                "message.todo",
                vec![
                    "message",
                    "todo",
                    &token,
                    "--state",
                    if mode == "todo-complete" {
                        "complete"
                    } else if mode == "todo-cancel" {
                        "cancel"
                    } else {
                        "open"
                    },
                ],
            )
        } else {
            (
                "message.essence",
                vec![
                    "message",
                    "essence",
                    &token,
                    "--enabled",
                    if mode == "essence-off" {
                        "false"
                    } else {
                        "true"
                    },
                ],
            )
        };
        let reading = mode.starts_with("signed");
        if !reading {
            args.extend(["--request-id", "group-content-example"]);
        }
        let server = server.start();
        let (c, v) = run(&args, &config);
        let success = matches!(
            mode,
            "essence-on"
                | "essence-off"
                | "todo-open"
                | "todo-complete"
                | "todo-cancel"
                | "todo-long"
                | "sign"
                | "signed"
        );
        let expected = if success {
            0
        } else if mode == "signed-bad-id" {
            3
        } else {
            4
        };
        assert_eq!(c, expected, "{mode}: {v}");
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&v),
            "{v}"
        );
        server.join().unwrap();
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(success || mode == "signed-bad-id")
        );
        if mode == "signed" {
            assert_eq!(v["data"]["complete"], false);
            assert_eq!(v["data"]["users"][0]["rank"], 1.5);
            assert_eq!(
                v["data"]["users"][0]["nativeTime"].as_f64(),
                Some(1700000000000.0)
            );
            assert!(v["data"]["users"][0].get("secret").is_none());
        }
        if success && !reading {
            let (c, v) = run(&args, &config);
            assert_eq!(c, 0, "{v}");
            assert_eq!(v["data"]["deduplicated"], true);
        }
        fn scan(p: &std::path::Path) {
            for e in std::fs::read_dir(p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    scan(&p);
                } else if let Ok(s) = std::fs::read_to_string(p) {
                    assert!(!s.contains("synthetic-private-content"));
                    assert!(!s.contains("synthetic-native-secret"));
                }
            }
        }
        scan(store.root());
    }
}

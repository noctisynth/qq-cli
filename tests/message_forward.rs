use serde_json::{Value, json};
use std::process::Command;
fn run(args: &[&str], config: &std::path::Path) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(out.stderr.is_empty());
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}
#[test]
fn forwarding_has_bounded_sources_and_no_implicit_scope() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.json");
    for args in [
        vec!["message", "forward", "synthetic-ref", "--dry-run"],
        vec![
            "message",
            "forward",
            "--conversation",
            "group:20003",
            "--dry-run",
        ],
        vec![
            "message",
            "forward",
            "synthetic-ref",
            "--conversation",
            "group:20003",
        ],
    ] {
        assert_eq!(run(&args, &missing).0, 2);
    }
    let mut args = vec!["message", "forward"];
    args.extend(std::iter::repeat_n("synthetic-ref", 33));
    args.extend(["--conversation", "group:20003", "--dry-run"]);
    assert_eq!(run(&args, &missing).0, 2);
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn owned_forwarding_checks_all_sources_and_recovers_only_minimal_receipts() {
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
        "single-group",
        "single-private",
        "merged-group",
        "merged-private",
        "explicit-merged",
        "cross-missing",
        "cross-allowed",
        "cross-denied",
        "dest-denied",
        "bad-result",
        "rejected",
        "source-changed",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-forward");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-forward", "synthetic-cli-token")
            .unwrap();
        let cross = mode.starts_with("cross-");
        let multiple =
            matches!(mode, "merged-group" | "merged-private" | "source-changed") || cross;
        let merged = multiple || matches!(mode, "explicit-merged" | "bad-result" | "rejected");
        let private = mode.ends_with("private");
        let count = if matches!(mode, "cross-missing" | "cross-denied" | "dest-denied") {
            0
        } else if mode == "source-changed" {
            17
        } else if multiple {
            21
        } else {
            14
        };
        let writes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let server_writes = writes.clone();
        let (endpoint, server) = http::serve(count, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_msg" => {
                    let id = payload["message_id"].as_str().unwrap();
                    assert!(matches!(id, "42" | "43"));
                    json!({"message_id":id,"message_type":"group","group_id":if cross&&id=="43"{20002}else{20001},"user_id":10002,"time":if mode=="source-changed"&&id=="43"{101}else{100},"real_seq":if id=="42"{7}else{8},"sender":{"user_id":10002,"nickname":"Synthetic"},"message":[{"type":"text","data":{"text":"synthetic-forward-body"}}]})
                }
                _ => {
                    server_writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let expected = if merged {
                        if private {
                            "send_private_forward_msg"
                        } else {
                            "send_group_forward_msg"
                        }
                    } else if private {
                        "forward_friend_single_msg"
                    } else {
                        "forward_group_single_msg"
                    };
                    assert_eq!(action, expected);
                    assert_eq!(
                        payload[if private { "user_id" } else { "group_id" }],
                        if private { "10003" } else { "20003" }
                    );
                    if merged {
                        let nodes = payload["message"].as_array().unwrap();
                        assert_eq!(nodes.len(), if multiple { 2 } else { 1 });
                        for (index, node) in nodes.iter().enumerate() {
                            assert_eq!(node["type"], "node");
                            assert_eq!(node["data"]["id"], if index == 0 { "42" } else { "43" });
                            assert_eq!(node["data"]["nickname"], "");
                            assert_eq!(node["data"]["content"], json!([]));
                        }
                        json!({"message_id":if mode=="bad-result"{0}else{8123},"res_id":"synthetic-result-secret"})
                    } else {
                        assert_eq!(payload["message_id"], "42");
                        Value::Null
                    }
                }
            };
            serde_json::to_vec(&json!({"status":if mode=="rejected"&&!matches!(action,"get_status"|"get_login_info"|"get_version_info"|"get_msg"){"failed"}else{"ok"},"retcode":if mode=="rejected"&&!matches!(action,"get_status"|"get_login_info"|"get_version_info"|"get_msg"){1400}else{0},"data":data})).unwrap()
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
            credential_key: "synthetic-forward".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        };
        tx.registry.instances.push(instance.clone());
        let mut other = instance;
        other.id = "unrelated-instance".into();
        other.credential_key = "untouched-credential".into();
        other.endpoint = "http://127.0.0.1:1/".into();
        tx.registry.instances.push(other);
        tx.commit().unwrap();
        let references =
            References::open(&store, &creds.0, Instant::now() + Duration::from_secs(5)).unwrap();
        let first = Locator {
            scope: Scope {
                instance_id: "instance-example".into(),
                account: "10001".into(),
            },
            run_id: "run-example".into(),
            conversation: "group:20001".parse().unwrap(),
            backend_id: "42".into(),
            time_seconds: Some(100),
            sequence: Some("7".into()),
            sender_id: Some("10002".into()),
        };
        let mut second = first.clone();
        second.backend_id = "43".into();
        second.sequence = Some("8".into());
        if cross {
            second.conversation = "group:20002".parse().unwrap();
        }
        let a = references.issue("m1", &first).unwrap();
        let b = references.issue("m1", &second).unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "owned_forwarding_checks_all_sources_and_recovers_only_minimal_receipts",
        );
        let server = server.start();
        let config = temp.path().join("config.json");
        let mut targets = vec!["group:20001"];
        if cross {
            targets.push("group:20002");
            if mode != "cross-denied" {
                targets.push("private:10001");
            }
        }
        if mode != "dest-denied" {
            targets.push(if private {
                "private:10003"
            } else {
                "group:20003"
            });
        }
        std::fs::write(
            &config,
            serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":targets}}))
                .unwrap(),
        )
        .unwrap();
        let mut args = vec!["message", "forward", a.as_str()];
        if multiple {
            args.push(&b);
        }
        args.extend([
            "--conversation",
            if private {
                "private:10003"
            } else {
                "group:20003"
            },
        ]);
        if merged && !multiple {
            args.push("--merged");
        }
        if cross && mode != "cross-missing" {
            args.push("--allow-self-copy");
        }
        let mut preview = args.clone();
        preview.push("--dry-run");
        let (code, planned) = run(&preview, &config);
        assert_eq!(
            code,
            if mode == "cross-missing" { 4 } else { 0 },
            "{mode}: {planned}"
        );
        if code == 0 {
            assert_eq!(planned["data"]["maySendSelfCopies"], cross);
            assert_eq!(
                planned["data"]["sourceMessages"],
                if multiple { 2 } else { 1 }
            );
        }
        args.extend(["--request-id", "forward-example"]);
        let (code, result) = run(&args, &config);
        let success = !matches!(
            mode,
            "cross-missing"
                | "cross-denied"
                | "dest-denied"
                | "bad-result"
                | "rejected"
                | "source-changed"
        );
        assert_eq!(
            code,
            if success {
                0
            } else if mode == "bad-result" {
                5
            } else {
                4
            },
            "{mode}: {result}"
        );
        assert!(!result.to_string().contains("synthetic-result-secret"));
        let schema = qq_cli::cli::registry::get("message.forward").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{mode}: {result}"
        );
        server.join().unwrap();
        let sent = usize::from(success || matches!(mode, "bad-result" | "rejected"));
        assert_eq!(writes.load(std::sync::atomic::Ordering::SeqCst), sent);
        if success {
            if merged {
                assert_eq!(
                    result["data"]["message"],
                    json!({"messageId":"8123","conversation":if private{"private:10003"}else{"group:20003"}})
                );
            }
            let (code, cached) = run(&args, &config);
            assert_eq!(code, 0, "{cached}");
            assert_eq!(cached["data"]["deduplicated"], true);
            if merged {
                assert_eq!(cached["data"]["message"], result["data"]["message"]);
            }
            let (code, recorded) = run(&["request", "get", "forward-example"], &config);
            assert_eq!(code, 0);
            if merged {
                assert_eq!(recorded["data"]["receipt"], result["data"]["message"]);
            }
            let schema = qq_cli::cli::registry::get("request.get").unwrap();
            assert!(
                jsonschema::validator_for(&schema["responseSchema"])
                    .unwrap()
                    .is_valid(&recorded)
            );
            if mode == "merged-group" {
                let mut changed = second.clone();
                changed.time_seconds = Some(99);
                let changed = references.issue("m1", &changed).unwrap();
                let (code, result) = run(
                    &[
                        "message",
                        "forward",
                        &a,
                        &changed,
                        "--conversation",
                        "group:20003",
                        "--request-id",
                        "forward-example",
                    ],
                    &config,
                );
                assert_eq!(code, 4);
                assert_eq!(result["error"]["code"], "REQUEST_CONFLICT");
                use sha2::Digest;
                let record_path = store.root().join("requests").join(format!(
                    "{:x}.json",
                    sha2::Sha256::digest(b"forward-example")
                ));
                let mut record: Value =
                    serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
                record["receipt"]["conversation"] = json!("group:29999");
                std::fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
                let (code, result) = run(&args, &config);
                assert_eq!(code, 6);
                assert_eq!(result["error"]["code"], "REQUEST_RECEIPT_INVALID");
            }
        }
        if mode == "bad-result" {
            let (code, result) = run(&args, &config);
            assert_eq!(code, 5);
            assert_eq!(result["error"]["code"], "WRITE_OUTCOME_UNKNOWN");
            let (code, record) = run(&["request", "get", "forward-example"], &config);
            assert_eq!(code, 0);
            assert_eq!(record["data"]["outcome"], "unknown");
            assert!(record["data"].get("receipt").is_none());
        }
        fn scan(path: &std::path::Path) {
            for entry in std::fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_dir() {
                    scan(&entry.path());
                } else if let Ok(text) = std::fs::read_to_string(entry.path()) {
                    assert!(!text.contains("synthetic-forward-body"));
                    assert!(!text.contains("synthetic-result-secret"));
                }
            }
        }
        scan(store.root());
    }
}

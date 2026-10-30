use serde_json::Value;
#[cfg(windows)]
use serde_json::json;
use std::process::{Command, Stdio};
fn run(args: &[&str], config: &std::path::Path) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}
#[test]
fn audiences_and_local_inputs_are_explicit_and_previews_do_not_initialize_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    for args in [
        vec!["qzone", "post", "--content", "private-data", "--dry-run"],
        vec![
            "qzone",
            "post",
            "--content",
            "private-data",
            "--audience",
            "selected",
            "--dry-run",
        ],
        vec![
            "qzone",
            "post",
            "--content",
            "private-data",
            "--audience",
            "friends",
            "--user-id",
            "10001",
            "--dry-run",
        ],
        vec![
            "qzone",
            "post",
            "--content",
            "private-data",
            "--audience",
            "exclude",
            "--user-id",
            "9007199254740993",
            "--dry-run",
        ],
        vec![
            "qzone",
            "post",
            "--content",
            "private-data",
            "--audience",
            "self",
        ],
        vec!["qzone", "delete", "synthetic-tid"],
    ] {
        assert_eq!(run(&args, &config).0, 2, "{args:?}");
    }
    for audience in ["everyone", "friends", "self", "selected", "exclude"] {
        let mut args = vec![
            "qzone",
            "post",
            "--content",
            "private-data",
            "--audience",
            audience,
            "--dry-run",
        ];
        if matches!(audience, "selected" | "exclude") {
            args.extend(["--user-id", "10001"]);
        }
        let (code, value) = run(&args, &config);
        assert_eq!(code, 0, "{value}");
        let schema = qq_cli::cli::registry::get("qzone.post").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&value)
        );
        assert!(!value.to_string().contains("private-data"));
    }
    assert_eq!(
        run(&["qzone", "delete", "synthetic-tid", "--dry-run"], &config).0,
        0
    );
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 0);
    for name in ["get_guild_list", "get_guild_service_profile"] {
        let (_, value) = run(&["api", "describe", name], &config);
        assert_eq!(value["data"]["compatibilityOnly"], true);
        assert_eq!(value["data"]["effect"], "read");
        assert_eq!(value["data"]["parametersKnown"], true);
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
fn managed_space_writes_preserve_audience_and_do_not_replay_or_save_content() {
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in [
        "post",
        "image",
        "selected",
        "delete",
        "rejected",
        "missing-tid",
        "changed",
        "scoped",
        "send-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-social");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let t = tempfile::tempdir().unwrap();
        let store = Store::new(t.path().join("state"));
        let vault = Cleanup(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-social", "synthetic-cli-token")
            .unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let preflight = matches!(mode, "scoped" | "send-denied");
        let (endpoint, server) = http::serve(
            if preflight { 0 } else { 7 },
            move |action, payload| {
                let data = match action {
                    "get_status" => json!({"online":true}),
                    "get_login_info" => {
                        json!({"user_id":if mode=="changed"&&observed.load(std::sync::atomic::Ordering::SeqCst)>0{"10002"}else{"10001"}})
                    }
                    "get_version_info" => json!({"app_version":"4.18.38"}),
                    _ => {
                        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        if mode == "delete" {
                            assert_eq!(action, "delete_qzone_msg");
                            assert_eq!(payload, &json!({"tid":"synthetic-tid"}));
                            Value::Null
                        } else {
                            assert_eq!(action, "send_qzone_msg");
                            assert_eq!(payload["content"], "synthetic-private-content");
                            assert_eq!(
                                payload["ugc_right"],
                                if mode == "selected" { 16.0 } else { 64.0 }
                            );
                            if mode == "selected" {
                                assert_eq!(payload["target_uins"], json!(["10003"]));
                            } else {
                                assert!(payload.get("target_uins").is_none());
                            }
                            if mode == "image" {
                                assert_eq!(
                                    payload["images"],
                                    json!(["base64://c3ludGhldGljLWltYWdl"])
                                );
                            } else {
                                assert!(payload.get("images").is_none());
                            }
                            if mode == "missing-tid" {
                                json!({"tid":""})
                            } else {
                                json!({"tid":"synthetic-native-tid","secret":"synthetic-native-secret"})
                            }
                        }
                    }
                };
                serde_json::to_vec(&json!({"status":if mode=="rejected"&&action=="send_qzone_msg"{"failed"}else{"ok"},"retcode":if mode=="rejected"&&action=="send_qzone_msg"{1200}else{0},"data":data})).unwrap()
            },
        );
        let mut tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        tx.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: t.path().join("unused"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-social".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &vault.0,
            "instance-example",
            "managed_space_writes_preserve_audience_and_do_not_replay_or_save_content",
        );
        let config = t.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"read":false,"send":mode!="send-denied","targets":if mode=="scoped"{vec!["group:20001"]}else{vec![]}}})).unwrap()).unwrap();
        let image = t.path().join("image.png");
        std::fs::write(&image, b"synthetic-image").unwrap();
        let mut args = if mode == "delete" {
            vec!["qzone", "delete", "synthetic-tid"]
        } else {
            vec![
                "qzone",
                "post",
                "--content",
                "synthetic-private-content",
                "--audience",
                if mode == "selected" {
                    "selected"
                } else {
                    "self"
                },
            ]
        };
        if mode == "image" {
            args.extend(["--image", image.to_str().unwrap()]);
        }
        if mode == "selected" {
            args.extend(["--user-id", "10003"]);
        }
        args.extend(["--request-id", "space-example"]);
        let server = server.start();
        let (code, value) = run(&args, &config);
        let expected = if preflight || mode == "rejected" {
            4
        } else if matches!(mode, "changed" | "missing-tid") {
            5
        } else {
            0
        };
        assert_eq!(code, expected, "{mode}: {value}");
        server.join().unwrap();
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(!preflight)
        );
        let path = if mode == "delete" {
            "qzone.delete"
        } else {
            "qzone.post"
        };
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&value),
            "{value}"
        );
        assert!(!value.to_string().contains("synthetic-native-secret"));
        if !preflight {
            let (cached, value) = run(&args, &config);
            assert_eq!(cached, code, "{mode}: {value}");
            if cached == 0 {
                assert_eq!(value["data"]["tidAvailable"], false);
                assert_eq!(value["data"]["deduplicated"], true);
            }
            let (_, record) = run(&["request", "get", "space-example"], &config);
            assert_eq!(record["data"]["resultAvailable"], false);
            let ledger = qq_cli::requests::ledger::get(&store, "space-example").unwrap();
            let text = serde_json::to_string(&ledger).unwrap();
            for secret in [
                "synthetic-private-content",
                "synthetic-native-tid",
                "synthetic-native-secret",
                "base64://",
            ] {
                assert!(!text.contains(secret), "{text}");
            }
        }
    }
}

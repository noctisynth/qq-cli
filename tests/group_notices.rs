use serde_json::Value;
#[cfg(windows)]
use serde_json::json;
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
fn announcement_plans_require_explicit_content_and_snapshot_only_local_bounded_files() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("missing.json");
    for args in [
        vec!["group", "notice", "20001", "--dry-run"],
        vec!["group", "notice", "20001", "--content", "synthetic"],
        vec!["group", "notice-delete", "20001", "synthetic-notice"],
        vec![
            "group",
            "essence-remove",
            "20001",
            "--sequence",
            "7",
            "--random",
            "9",
        ],
    ] {
        assert_eq!(run(&args, &config).0, 2);
    }
    let (c, v) = run(
        &[
            "group",
            "notice",
            "20001",
            "--content",
            "synthetic-private-body",
            "--confirm-required",
            "false",
            "--dry-run",
        ],
        &config,
    );
    assert_eq!(c, 0, "{v}");
    assert_eq!(v["data"]["identityVerified"], false);
    assert!(!v.to_string().contains("synthetic-private-body"));
    assert_eq!(
        run(
            &[
                "group",
                "essence-remove",
                "20001",
                "--sequence",
                "7",
                "--random=-9",
                "--dry-run"
            ],
            &config
        )
        .0,
        0
    );
    assert_eq!(
        run(
            &[
                "group",
                "essence-remove",
                "20001",
                "--sequence",
                "9007199254740993",
                "--random",
                "9",
                "--dry-run"
            ],
            &config
        )
        .0,
        2
    );
    let path = temp.path().join("oversized.png");
    std::fs::File::create(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    let (c, v) = run(
        &[
            "group",
            "notice",
            "20001",
            "--image",
            path.to_str().unwrap(),
            "--dry-run",
        ],
        &config,
    );
    assert_eq!(c, 2);
    assert_eq!(v["error"]["code"], "NOTICE_IMAGE_INPUT");
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
fn managed_announcements_and_essence_snapshots_keep_source_authority_separate() {
    use base64::Engine;
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in [
        "notices",
        "essences",
        "bad-essence",
        "publish",
        "publish-image",
        "delete",
        "remove",
        "publish-reject",
        "publish-send-only",
        "target-denied",
        "read-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-group-notices");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-group-notices", "synthetic-cli-token")
            .unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let expected = if mode.ends_with("denied") { 0 } else { 7 };
        let (endpoint, server) = http::serve(expected, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                _ => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let wanted = match mode {
                        "notices" | "read-denied" => "_get_group_notice",
                        "essences" | "bad-essence" => "get_essence_msg_list",
                        "delete" => "_del_group_notice",
                        "remove" => "delete_essence_msg",
                        _ => "_send_group_notice",
                    };
                    assert_eq!(action, wanted);
                    if matches!(mode, "notices" | "essences" | "bad-essence") {
                        assert_eq!(payload, &json!({"group_id":"20001"}));
                    }
                    if mode == "notices" {
                        json!([{"sender_id":10002,"publish_time":1700000000,"notice_id":"synthetic-notice","message":{"text":"synthetic-private-body","images":[{"id":"opaque-image","width":1,"height":1}]},"settings":{"secret":"synthetic-native-secret"},"read_num":0,"send_to_new_member":true,"pinned":false},{"sender_id":10002,"publish_time":1700000001,"notice_id":"ignored-by-limit","message":{"text":"synthetic-extra","images":[]}}])
                    } else if matches!(mode, "essences" | "bad-essence") {
                        json!([{"msg_seq":if mode=="bad-essence"{9_007_199_254_740_993_u64}else{7},"msg_random":-9,"message_id":-42,"sender_id":10002,"sender_nick":"Synthetic","operator_id":10001,"operator_nick":"Synthetic operator","operator_time":1700000000,"content":[{"type":"text","data":{"text":"synthetic-private-body"}},{"type":"image","data":{"file":"opaque-image","file_id":"image-id","url":"https://invalid.invalid/image?secret=synthetic-native-secret"}},{"type":"reply","data":{"id":"42"}},{"type":"record","data":{"file":"opaque-voice.silk"}},{"type":"forward","data":{"id":"synthetic-native-resource"}},{"type":"file","data":{"file":"doc.txt","file_id":"opaque-file"}}]}])
                    } else {
                        match mode {
                            "delete" => assert_eq!(
                                payload,
                                &json!({"group_id":"20001","notice_id":"synthetic-notice"})
                            ),
                            "remove" => assert_eq!(
                                payload,
                                &json!({"group_id":"20001","msg_seq":"7","msg_random":"-9"})
                            ),
                            _ => {
                                assert_eq!(payload["group_id"], "20001");
                                assert_eq!(
                                    payload["confirm_required"],
                                    if mode == "publish-image" {
                                        json!(0.0)
                                    } else {
                                        json!(1.0)
                                    }
                                );
                                assert_eq!(
                                    payload["content"],
                                    if mode == "publish-image" {
                                        ""
                                    } else {
                                        "synthetic-private-body"
                                    }
                                );
                                if mode == "publish-image" {
                                    assert_eq!(payload["pinned"], json!(1.0));
                                    let encoded = payload["image"]
                                        .as_str()
                                        .unwrap()
                                        .strip_prefix("base64://")
                                        .unwrap();
                                    assert_eq!(
                                        base64::engine::general_purpose::STANDARD
                                            .decode(encoded)
                                            .unwrap(),
                                        b"synthetic-local-image-bytes"
                                    );
                                } else {
                                    assert!(payload.get("image").is_none());
                                }
                            }
                        }
                        Value::Null
                    }
                }
            };
            serde_json::to_vec(&json!({"status":if mode=="publish-reject"&&action=="_send_group_notice"{"failed"}else{"ok"},"retcode":if mode=="publish-reject"&&action=="_send_group_notice"{1200}else{0},"data":data})).unwrap()
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
            credential_key: "synthetic-group-notices".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "managed_announcements_and_essence_snapshots_keep_source_authority_separate",
        );
        let config = temp.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[if mode=="target-denied"{"group:20002"}else{"group:20001"}],"read":!matches!(mode,"read-denied"|"publish-send-only")}})).unwrap()).unwrap();
        let image = temp.path().join("image.png");
        std::fs::write(&image, b"synthetic-local-image-bytes").unwrap();
        let (path, mut args) = match mode {
            "notices" | "read-denied" => (
                "group.notices",
                vec!["group", "notices", "20001", "--limit", "1"],
            ),
            "essences" | "bad-essence" => (
                "group.essence-list",
                vec!["group", "essence-list", "20001", "--limit", "1"],
            ),
            "delete" => (
                "group.notice-delete",
                vec!["group", "notice-delete", "20001", "synthetic-notice"],
            ),
            "remove" => (
                "group.essence-remove",
                vec![
                    "group",
                    "essence-remove",
                    "20001",
                    "--sequence",
                    "7",
                    "--random=-9",
                ],
            ),
            "publish-image" => (
                "group.notice",
                vec![
                    "group",
                    "notice",
                    "20001",
                    "--image",
                    image.to_str().unwrap(),
                    "--pinned",
                    "--confirm-required",
                    "false",
                ],
            ),
            _ => (
                "group.notice",
                vec![
                    "group",
                    "notice",
                    "20001",
                    "--content",
                    "synthetic-private-body",
                ],
            ),
        };
        let read = matches!(mode, "notices" | "read-denied" | "essences" | "bad-essence");
        if !read {
            args.extend(["--request-id", "group-notice-example"]);
        }
        let server = server.start();
        let (c, v) = run(&args, &config);
        let expected = if mode == "bad-essence" {
            3
        } else if mode.ends_with("denied") || mode == "publish-reject" {
            4
        } else {
            0
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
            usize::from(!mode.ends_with("denied"))
        );
        if mode == "notices" {
            assert_eq!(v["data"]["complete"], false);
            assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
            assert_eq!(v["data"]["items"][0]["pinned"], false);
            assert!(v["data"]["items"][0].get("settings").is_none());
        }
        if mode == "essences" {
            assert_eq!(v["data"]["complete"], false);
            assert_eq!(v["data"]["items"][0]["random"], "-9");
            let segments = v["data"]["items"][0]["segments"].as_array().unwrap();
            assert_eq!(segments.len(), 6);
            let output = v.to_string();
            for absent in [
                "mediaRef",
                "messageRef",
                "synthetic-native-secret",
                "synthetic-native-resource",
            ] {
                assert!(!output.contains(absent));
            }
        }
        if !read && !mode.ends_with("denied") {
            let (cached, v) = run(&args, &config);
            assert_eq!(cached, c, "{v}");
            if c == 0 {
                assert_eq!(v["data"]["deduplicated"], true);
            }
        }
        fn scan(p: &std::path::Path) {
            for e in std::fs::read_dir(p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    scan(&p);
                } else if let Ok(s) = std::fs::read_to_string(p) {
                    assert!(
                        !s.contains(
                            &base64::engine::general_purpose::STANDARD
                                .encode(b"synthetic-local-image-bytes")
                        )
                    );
                    for secret in [
                        "synthetic-private-body",
                        "synthetic-native-secret",
                        "synthetic-local-image-bytes",
                    ] {
                        assert!(!s.contains(secret));
                    }
                }
            }
        }
        scan(store.root());
    }
}

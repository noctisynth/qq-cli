use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};
fn run(args: &[&str], config: &std::path::Path, input: Option<&[u8]>) -> (i32, Value) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(input) = input {
        c.stdin.take().unwrap().write_all(input).unwrap();
    }
    let o = c.wait_with_output().unwrap();
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    (
        o.status.code().unwrap(),
        serde_json::from_slice(&o.stdout).unwrap(),
    )
}
#[test]
fn repository_inputs_are_explicit_bounded_and_do_not_initialize_offline_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    for args in [
        vec!["favorite", "create", "--brief", "synthetic", "--dry-run"],
        vec![
            "favorite",
            "create",
            "--brief",
            "synthetic",
            "--raw-data",
            "body",
        ],
        vec![
            "favorite",
            "create",
            "--brief",
            "synthetic",
            "--raw-data",
            "body",
            "--stdin",
            "--dry-run",
        ],
        vec!["emoji", "remove", "synthetic-resource"],
        vec![
            "emoji",
            "describe",
            "synthetic-resource",
            "--emoji-id",
            "1",
            "--md5",
            "synthetic-md5",
            "--description",
            "changed",
        ],
    ] {
        assert_eq!(run(&args, &config, None).0, 2);
    }
    let file = t.path().join("data.txt");
    std::fs::write(&file, "synthetic-private-data").unwrap();
    for (args, input) in [
        (
            vec![
                "favorite",
                "create",
                "--brief",
                "synthetic",
                "--raw-data",
                "synthetic-private-data",
                "--dry-run",
            ],
            None,
        ),
        (
            vec![
                "favorite",
                "create",
                "--brief",
                "synthetic",
                "--raw-file",
                file.to_str().unwrap(),
                "--dry-run",
            ],
            None,
        ),
        (
            vec![
                "favorite",
                "create",
                "--brief",
                "synthetic",
                "--stdin",
                "--dry-run",
            ],
            Some(b"synthetic-private-data".as_slice()),
        ),
        (
            vec!["emoji", "remove", "synthetic-resource", "--dry-run"],
            None,
        ),
        (
            vec![
                "emoji",
                "describe",
                "synthetic-resource",
                "--emoji-id",
                "0",
                "--md5",
                "synthetic-md5",
                "--description=",
                "--dry-run",
            ],
            None,
        ),
    ] {
        let (c, v) = run(&args, &config, input);
        assert_eq!(c, 0, "{v}");
        assert_eq!(v["data"]["identityVerified"], false);
        assert!(!v.to_string().contains("synthetic-private-data"));
    }
    std::fs::write(&file, [255]).unwrap();
    assert_eq!(
        run(
            &[
                "favorite",
                "create",
                "--brief",
                "synthetic",
                "--raw-file",
                file.to_str().unwrap(),
                "--dry-run"
            ],
            &config,
            None
        )
        .0,
        2
    );
    std::fs::write(&file, "\"".repeat(700_000)).unwrap();
    let (c, v) = run(
        &[
            "favorite",
            "create",
            "--brief",
            "synthetic",
            "--raw-file",
            file.to_str().unwrap(),
            "--dry-run",
        ],
        &config,
        None,
    );
    assert_eq!(c, 2);
    assert_eq!(v["error"]["code"], "INPUT_LIMIT");
    assert_eq!(
        run(
            &[
                "emoji",
                "describe",
                "synthetic-resource",
                "--emoji-id",
                "9007199254740993",
                "--md5",
                "synthetic-md5",
                "--description",
                "changed",
                "--dry-run"
            ],
            &config,
            None
        )
        .0,
        2
    );
    assert!(!t.path().join("state").exists());
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_repositories_bound_summaries_preserve_identity_and_never_replay_writes() {
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in [
        "favorites",
        "favorites-error",
        "favorites-conflict-code",
        "favorites-scoped",
        "emoji-list",
        "emoji-wide-id",
        "create",
        "create-file",
        "create-stdin",
        "create-reject",
        "remove",
        "describe",
        "read-denied",
        "send-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-assets");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let t = tempfile::tempdir().unwrap();
        let store = Store::new(t.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-assets", "synthetic-cli-token")
            .unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let expected = if matches!(mode, "favorites-scoped" | "read-denied" | "send-denied") {
            0
        } else {
            7
        };
        let (endpoint, server) = http::serve(expected, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                _ => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let wanted = match mode {
                        "favorites" | "favorites-error" | "favorites-conflict-code" => {
                            "get_collection_list"
                        }
                        "emoji-list" | "emoji-wide-id" => "fetch_custom_face_detail",
                        "remove" => "delete_custom_face",
                        "describe" => "set_custom_face_desc",
                        _ => "create_collection",
                    };
                    assert_eq!(action, wanted);
                    if mode.starts_with("favorites") {
                        assert_eq!(payload, &json!({"category":"0","count":"1"}));
                        let mut result = json!({"errCode":if mode=="favorites-error"{17}else{0},"collectionSearchList":{"hasMore":true,"bottomTimeStamp":"synthetic-private-timestamp","collectionItemList":[{"cid":"synthetic-item","type":8,"status":1,"category":2,"createTime":"1700000000000","collectTime":"1700000000001","modifyTime":"1700000000002","author":{"secret":"synthetic-native-secret"},"summary":{"richMediaSummary":{"title":"synthetic-title","brief":"synthetic-private-data"},"textSummary":{"secret":"synthetic-native-secret"}}},{"cid":"extra-item","type":8,"status":1,"category":2,"createTime":"1","collectTime":"1","modifyTime":"1"}]}});
                        if mode == "favorites-conflict-code" {
                            result["result"] = json!(17);
                        }
                        result
                    } else if matches!(mode, "emoji-list" | "emoji-wide-id") {
                        assert_eq!(payload, &json!({"count":1.0}));
                        json!([{"resId":"synthetic-resource","emoId":if mode=="emoji-wide-id"{9_007_199_254_740_993_i64}else{0},"md5":"synthetic-md5","desc":"synthetic-description","url":"https://invalid.invalid/emoji","isExist":false,"isAPNG":false,"isMarkFace":false,"emoPath":"synthetic-private-path","emoOriginalPath":"synthetic-private-path","thumbPath":"synthetic-private-path","ocrWord":"synthetic-native-secret"}])
                    } else {
                        match mode {
                            "remove" => assert_eq!(
                                payload,
                                &json!({"ids":["synthetic-resource","synthetic-resource-2"]})
                            ),
                            "describe" => assert_eq!(
                                payload,
                                &json!({"res_id":"synthetic-resource","emoji_id":"0","md5":"synthetic-md5","desc":""})
                            ),
                            _ => assert_eq!(
                                payload,
                                &json!({"brief":"synthetic-title","rawData":"synthetic-private-data"})
                            ),
                        }
                        json!({"syntheticResult":"synthetic-native-secret"})
                    }
                }
            };
            serde_json::to_vec(&json!({"status":if mode=="create-reject"&&action=="create_collection"{"failed"}else{"ok"},"retcode":if mode=="create-reject"&&action=="create_collection"{1200}else{0},"data":data})).unwrap()
        });
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
            credential_key: "synthetic-assets".into(),
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
            "managed_repositories_bound_summaries_preserve_identity_and_never_replay_writes",
        );
        let config = t.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":if mode=="favorites-scoped"{vec!["group:20001"]}else{vec![]},"read":mode!="read-denied","send":mode!="send-denied"}})).unwrap()).unwrap();
        let file = t.path().join("favorite.txt");
        std::fs::write(&file, "synthetic-private-data").unwrap();
        let (path, mut args, input) = if mode.starts_with("favorites") {
            (
                "favorite.list",
                vec!["favorite", "list", "--limit", "1"],
                None,
            )
        } else if matches!(mode, "emoji-list" | "emoji-wide-id" | "read-denied") {
            ("emoji.list", vec!["emoji", "list", "--limit", "1"], None)
        } else if mode == "remove" {
            (
                "emoji.remove",
                vec![
                    "emoji",
                    "remove",
                    "synthetic-resource",
                    "synthetic-resource-2",
                ],
                None,
            )
        } else if mode == "describe" {
            (
                "emoji.describe",
                vec![
                    "emoji",
                    "describe",
                    "synthetic-resource",
                    "--emoji-id",
                    "0",
                    "--md5",
                    "synthetic-md5",
                    "--description=",
                ],
                None,
            )
        } else if mode == "create-file" {
            (
                "favorite.create",
                vec![
                    "favorite",
                    "create",
                    "--brief",
                    "synthetic-title",
                    "--raw-file",
                    file.to_str().unwrap(),
                ],
                None,
            )
        } else if mode == "create-stdin" {
            (
                "favorite.create",
                vec![
                    "favorite",
                    "create",
                    "--brief",
                    "synthetic-title",
                    "--stdin",
                ],
                Some(b"synthetic-private-data".as_slice()),
            )
        } else {
            (
                "favorite.create",
                vec![
                    "favorite",
                    "create",
                    "--brief",
                    "synthetic-title",
                    "--raw-data",
                    "synthetic-private-data",
                ],
                None,
            )
        };
        let writing = matches!(path, "favorite.create" | "emoji.remove" | "emoji.describe");
        if writing {
            args.extend(["--request-id", "asset-example"]);
        }
        let server = server.start();
        let (c, v) = run(&args, &config, input);
        let failed = matches!(
            mode,
            "favorites-error"
                | "favorites-conflict-code"
                | "favorites-scoped"
                | "create-reject"
                | "read-denied"
                | "send-denied"
        );
        assert_eq!(c, if failed { 4 } else { 0 }, "{mode}: {v}");
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
            usize::from(expected != 0)
        );
        if mode == "favorites" {
            assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
            assert_eq!(v["data"]["complete"], false);
            assert_eq!(v["data"]["bodyAvailable"], false);
            assert_eq!(v["data"]["hasMore"], true);
        }
        if mode == "emoji-list" {
            assert_eq!(v["data"]["items"][0]["nativeExists"], false);
            assert_eq!(v["data"]["items"][0]["emojiId"], 0);
            for forbidden in [
                "synthetic-private-path",
                "synthetic-native-secret",
                "ocrWord",
            ] {
                assert!(!v.to_string().contains(forbidden));
            }
        }
        if mode == "emoji-wide-id" {
            assert!(v["data"]["items"][0].get("emojiId").is_none());
        }
        if writing && mode != "send-denied" {
            let (code, cached) = run(&args, &config, input);
            assert_eq!(code, c, "{cached}");
            if !failed {
                assert_eq!(cached["data"]["deduplicated"], true);
            }
        }
        fn scan(p: &std::path::Path) {
            for e in std::fs::read_dir(p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    scan(&p);
                } else if let Ok(s) = std::fs::read_to_string(p) {
                    for secret in [
                        "synthetic-private-data",
                        "synthetic-private-path",
                        "synthetic-native-secret",
                    ] {
                        assert!(!s.contains(secret));
                    }
                }
            }
        }
        scan(store.root());
    }
}

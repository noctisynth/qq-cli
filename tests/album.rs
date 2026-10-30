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
fn album_previews_require_explicit_keys_flags_and_local_files_without_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    let image = t.path().join("image.png");
    std::fs::write(&image, b"synthetic-album-source").unwrap();
    for args in [
        vec!["album", "remove", "20001", "album-example", "lloc-example"],
        vec![
            "album",
            "like",
            "20001",
            "album-example",
            "batch-example",
            "--dry-run",
        ],
        vec![
            "album",
            "upload",
            "20001",
            "album-example",
            "--album-name",
            "name",
            "--dry-run",
        ],
    ] {
        assert_eq!(run(&args, &config).0, 2, "{args:?}");
    }
    for (path, args) in [
        (
            "album.remove",
            vec![
                "album",
                "remove",
                "20001",
                "album-example",
                "lloc-example",
                "--dry-run",
            ],
        ),
        (
            "album.like",
            vec![
                "album",
                "like",
                "20001",
                "album-example",
                "batch-example",
                "--enabled",
                "false",
                "--dry-run",
            ],
        ),
        (
            "album.comment",
            vec![
                "album",
                "comment",
                "20001",
                "album-example",
                "lloc-example",
                "--content",
                "synthetic-private-comment",
                "--dry-run",
            ],
        ),
        (
            "album.upload",
            vec![
                "album",
                "upload",
                "20001",
                "album-example",
                "--album-name",
                "name",
                "--file",
                image.to_str().unwrap(),
                "--dry-run",
            ],
        ),
        (
            "album.upload-video",
            vec![
                "album",
                "upload-video",
                "20001",
                "album-example",
                image.to_str().unwrap(),
                "--album-name",
                "name",
                "--dry-run",
            ],
        ),
    ] {
        let (code, value) = run(&args, &config);
        assert_eq!(code, 0, "{value}");
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&value)
        );
        assert!(!value.to_string().contains("synthetic-private-comment"));
    }
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 1);
    let mut many = vec![
        "album",
        "upload",
        "20001",
        "album-example",
        "--album-name",
        "name",
        "--dry-run",
    ];
    for _ in 0..33 {
        many.extend(["--file", image.to_str().unwrap()]);
    }
    assert_eq!(run(&many, &config).0, 2);
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_albums_enforce_group_scope_native_failures_and_content_bound_batch_lifetimes() {
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        references::References,
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant, SystemTime};
    if managed::child() {
        return;
    }
    for mode in [
        "list",
        "list-truncated",
        "media",
        "media-failed",
        "media-unsupported",
        "remove",
        "like",
        "unlike",
        "comment",
        "native-reject",
        "image",
        "batch",
        "video",
        "batch-changed",
        "target-denied",
        "read-denied",
        "send-denied",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-album");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let t = tempfile::tempdir().unwrap();
        let store = Store::new(t.path().join("state"));
        let vault = Cleanup(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-album", "synthetic-cli-token")
            .unwrap();
        let reads = matches!(
            mode,
            "list"
                | "list-truncated"
                | "media"
                | "media-failed"
                | "media-unsupported"
                | "read-denied"
        );
        let blocked = matches!(mode, "target-denied" | "read-denied" | "send-denied");
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let root = store.root().to_owned();
        let (endpoint, server) = http::serve(
            if blocked { 0 } else { 7 },
            move |action, payload| {
                let data = match action {
                    "get_status" => json!({"online":true}),
                    "get_login_info" => {
                        json!({"user_id":if mode=="batch-changed"&&observed.load(std::sync::atomic::Ordering::SeqCst)>0{"10002"}else{"10001"}})
                    }
                    "get_version_info" => json!({"app_version":"4.18.38"}),
                    _ => {
                        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        assert_eq!(payload["group_id"], "20001");
                        match mode {
                            "list" | "list-truncated" => {
                                assert_eq!(action, "get_qun_album_list");
                                assert_eq!(payload["attach_info"], "page-input");
                                json!({"album_list":[{"album_id":"album-example","name":"native-name","album_name":"example-alias","secret":"native-secret"},{"album_id":"album-extra","name":"extra"}],"attach_info":"next-page","has_more":true})
                            }
                            "media" | "media-failed" | "media-unsupported" => {
                                assert_eq!(action, "get_group_album_media_list");
                                assert_eq!(payload["album_id"], "album-example");
                                if mode == "media-unsupported" {
                                    json!({"result":0})
                                } else {
                                    json!({"result":if mode=="media-failed"{7}else{0},"media_list":[{"media_id":"media-example","url":"https://invalid.invalid/album","secret":"native-secret"}],"attach_info":"next-media"})
                                }
                            }
                            "remove" | "native-reject" => {
                                assert_eq!(action, "del_group_album_media");
                                assert_eq!(payload["lloc"], "lloc-example");
                                json!({"result":if mode=="native-reject"{7}else{0},"secret":"native-secret"})
                            }
                            "like" | "unlike" => {
                                assert_eq!(
                                    action,
                                    if mode == "like" {
                                        "set_group_album_media_like"
                                    } else {
                                        "cancel_group_album_media_like"
                                    }
                                );
                                assert_eq!(payload["batch_id"], "batch-example");
                                assert_eq!(
                                    payload.get("lloc").and_then(Value::as_str),
                                    if mode == "like" {
                                        Some("lloc-example")
                                    } else {
                                        None
                                    }
                                );
                                json!({"result":0})
                            }
                            "comment" => {
                                assert_eq!(action, "do_group_album_comment");
                                assert_eq!(payload["content"], "synthetic-private-comment");
                                json!({"result":0})
                            }
                            _ => {
                                assert_eq!(
                                    action,
                                    if mode.starts_with("batch") {
                                        "upload_images_to_qun_album"
                                    } else if mode == "video" {
                                        "upload_video_to_qun_album"
                                    } else {
                                        "upload_image_to_qun_album"
                                    }
                                );
                                assert_eq!(payload["album_name"], "synthetic-album");
                                let paths = if let Some(paths) = payload.get("files") {
                                    paths.as_array().unwrap().clone()
                                } else {
                                    vec![payload["file"].clone()]
                                };
                                assert_eq!(
                                    paths.len(),
                                    if mode.starts_with("batch") { 2 } else { 1 }
                                );
                                for value in paths {
                                    let path = std::path::PathBuf::from(value.as_str().unwrap());
                                    assert!(path.canonicalize().unwrap().starts_with(
                                        root.join("attachments").canonicalize().unwrap()
                                    ));
                                    assert_eq!(
                                        std::fs::read(&path).unwrap(),
                                        b"synthetic-album-source"
                                    );
                                    let pointer: Value = serde_json::from_slice(
                                        &std::fs::read(
                                            path.parent().unwrap().join(".qq-cli-attachment.json"),
                                        )
                                        .unwrap(),
                                    )
                                    .unwrap();
                                    let references = References::existing(
                                        &Store::new(root.clone()),
                                        &SystemCredentials::new(&root).unwrap(),
                                    )
                                    .unwrap();
                                    let owned: Value = references
                                        .get("a1", pointer["reference"].as_str().unwrap())
                                        .unwrap();
                                    assert_eq!(owned.get("persistent"), None);
                                    let ledger = qq_cli::requests::ledger::get(
                                        &Store::new(root.clone()),
                                        "album-example",
                                    )
                                    .unwrap();
                                    assert_eq!(owned["intentPayloadHash"], ledger.payload_hash);
                                    assert_eq!(
                                        ledger.outcome,
                                        qq_cli::core::contracts::write::Outcome::Pending
                                    );
                                }
                                Value::Null
                            }
                        }
                    }
                };
                serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
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
            credential_key: "synthetic-album".into(),
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
            "managed_albums_enforce_group_scope_native_failures_and_content_bound_batch_lifetimes",
        );
        let config = t.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[if mode=="target-denied"{"group:20002"}else{"group:20001"}],"read":mode!="read-denied","send":mode!="send-denied"}})).unwrap()).unwrap();
        let image = t.path().join("image.png");
        let second = t.path().join("second.png");
        for p in [&image, &second] {
            std::fs::write(p, b"synthetic-album-source").unwrap();
        }
        let (path, mut args) = match mode {
            "list" | "list-truncated" | "read-denied" => (
                "album.list",
                vec![
                    "album",
                    "list",
                    "20001",
                    "--attach-info",
                    "page-input",
                    "--limit",
                    if mode == "list-truncated" { "1" } else { "2" },
                ],
            ),
            "media" | "media-failed" | "media-unsupported" => (
                "album.media",
                vec!["album", "media", "20001", "album-example"],
            ),
            "remove" | "native-reject" | "target-denied" | "send-denied" => (
                "album.remove",
                vec!["album", "remove", "20001", "album-example", "lloc-example"],
            ),
            "like" => (
                "album.like",
                vec![
                    "album",
                    "like",
                    "20001",
                    "album-example",
                    "batch-example",
                    "--enabled",
                    "true",
                    "--lloc",
                    "lloc-example",
                ],
            ),
            "unlike" => (
                "album.like",
                vec![
                    "album",
                    "like",
                    "20001",
                    "album-example",
                    "batch-example",
                    "--enabled",
                    "false",
                ],
            ),
            "comment" => (
                "album.comment",
                vec![
                    "album",
                    "comment",
                    "20001",
                    "album-example",
                    "lloc-example",
                    "--content",
                    "synthetic-private-comment",
                ],
            ),
            "video" => (
                "album.upload-video",
                vec![
                    "album",
                    "upload-video",
                    "20001",
                    "album-example",
                    image.to_str().unwrap(),
                    "--album-name",
                    "synthetic-album",
                ],
            ),
            _ => {
                let mut args = vec![
                    "album",
                    "upload",
                    "20001",
                    "album-example",
                    "--album-name",
                    "synthetic-album",
                    "--file",
                    image.to_str().unwrap(),
                ];
                if mode.starts_with("batch") {
                    args.extend(["--file", second.to_str().unwrap()]);
                }
                ("album.upload", args)
            }
        };
        if !reads {
            args.extend(["--request-id", "album-example"]);
        }
        let server = server.start();
        let (code, value) = run(&args, &config);
        let expected = if blocked || matches!(mode, "media-failed" | "native-reject") {
            4
        } else if mode == "media-unsupported" {
            3
        } else if mode == "batch-changed" {
            5
        } else {
            0
        };
        assert_eq!(code, expected, "{mode}: {value}");
        server.join().unwrap();
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(!blocked)
        );
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&value),
            "{value}"
        );
        assert!(!value.to_string().contains("native-secret"));
        if mode == "list" {
            assert_eq!(value["data"]["items"][0]["name"], "native-name");
            assert_eq!(value["data"]["nextAttachInfo"], "next-page");
        }
        if mode == "list-truncated" {
            assert_eq!(value["data"]["truncated"], true);
            assert!(value["data"].get("nextAttachInfo").is_none());
        }
        if mode == "media" {
            assert_eq!(value["data"]["items"][0]["mediaId"], "media-example");
            assert!(!value.to_string().contains("mediaRef"));
        }
        if !reads && !blocked {
            let (cached, value) = run(&args, &config);
            assert_eq!(cached, code, "{mode}: {value}");
            if mode.starts_with("batch") {
                std::fs::write(&second, b"changed-source").unwrap();
                let (code, value) = run(&args, &config);
                assert_eq!(code, 4, "{value}");
                assert_eq!(value["error"]["code"], "REQUEST_CONFLICT");
            }
            let ledger = qq_cli::requests::ledger::get(&store, "album-example").unwrap();
            let text = serde_json::to_string(&ledger).unwrap();
            for secret in [
                "synthetic-album-source",
                "synthetic-private-comment",
                "image.png",
                "second.png",
            ] {
                assert!(!text.contains(secret));
            }
            if matches!(mode, "image" | "batch" | "video" | "batch-changed") {
                let refs = References::existing(&store, &vault.0).unwrap();
                let result = qq_cli::messaging::attachments::cache::prune(
                    &store,
                    &refs,
                    None,
                    SystemTime::now() + Duration::from_secs(2 * 24 * 60 * 60),
                    Instant::now() + Duration::from_secs(5),
                )
                .unwrap();
                let result = serde_json::to_value(result).unwrap();
                assert_eq!(
                    result["data"]["removedFiles"],
                    if mode == "batch-changed" {
                        0
                    } else if mode == "batch" {
                        2
                    } else {
                        1
                    }
                );
            }
        }
    }
}

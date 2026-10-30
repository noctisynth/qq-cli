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
fn online_previews_use_native_ids_and_explicit_source_kinds_without_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    let file = t.path().join("input.bin");
    std::fs::write(&file, b"synthetic-online-input").unwrap();
    let dir = t.path().join("input-dir");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("data"), b"synthetic-online-input").unwrap();
    for (path, args) in [
        (
            "online.send",
            vec![
                "online",
                "send",
                "10002",
                file.to_str().unwrap(),
                "--dry-run",
            ],
        ),
        (
            "online.send-folder",
            vec![
                "online",
                "send-folder",
                "10002",
                dir.to_str().unwrap(),
                "--dry-run",
            ],
        ),
        (
            "online.receive",
            vec![
                "online",
                "receive",
                "10002",
                "native-msg",
                "native-element",
                "--dry-run",
            ],
        ),
        (
            "online.refuse",
            vec![
                "online",
                "refuse",
                "10002",
                "native-msg",
                "native-element",
                "--dry-run",
            ],
        ),
        (
            "online.cancel",
            vec!["online", "cancel", "10002", "native-msg", "--dry-run"],
        ),
    ] {
        let (code, v) = run(&args, &config);
        assert_eq!(code, 0, "{v}");
        assert!(
            jsonschema::validator_for(&qq_cli::cli::registry::get(path).unwrap()["responseSchema"])
                .unwrap()
                .is_valid(&v)
        );
    }
    for args in [
        vec!["online", "send", "10002", file.to_str().unwrap()],
        vec![
            "online",
            "send-folder",
            "10002",
            file.to_str().unwrap(),
            "--dry-run",
        ],
        vec![
            "online",
            "send",
            "10002",
            dir.to_str().unwrap(),
            "--dry-run",
        ],
        vec![
            "online",
            "receive",
            "10002",
            "m1.synthetic.token",
            "native-element",
            "--dry-run",
        ],
        vec![
            "online",
            "refuse",
            "10002",
            "native-msg",
            "f1.synthetic.token",
            "--dry-run",
        ],
        vec![
            "online",
            "send",
            "group:20001",
            file.to_str().unwrap(),
            "--dry-run",
        ],
    ] {
        assert_eq!(run(&args, &config).0, 2, "{args:?}");
    }
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 2);
    for action in [
        "send_online_file",
        "send_online_folder",
        "get_online_file_msg",
        "receive_online_file",
        "refuse_online_file",
        "cancel_online_file",
    ] {
        let (_, v) = run(&["api", "describe", action], &config);
        assert_eq!(v["data"]["parametersKnown"], true);
        assert_eq!(
            v["data"]["effect"],
            if action == "get_online_file_msg" {
                "read"
            } else {
                "write"
            }
        );
    }
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
mod owned {
    use super::*;
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        references::References,
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    struct Vault(SystemCredentials);
    impl Drop for Vault {
        fn drop(&mut self) {
            let _ = self.0.delete("synthetic-online");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn setup(t: &tempfile::TempDir, endpoint: String) -> (Store, Vault, managed::Managed) {
        let store = Store::new(t.path().join("state"));
        let vault = Vault(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-online", "synthetic-cli-token")
            .unwrap();
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
            credential_key: "synthetic-online".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let owner = managed::Managed::attach(
            &store,
            &vault.0,
            "instance-example",
            "owned::online_transfers_bind_private_peers_sources_and_native_ids",
        );
        (store, vault, owner)
    }

    #[test]
    fn online_transfers_bind_private_peers_sources_and_native_ids() {
        if managed::child() {
            return;
        }
        for mode in [
            "send",
            "folder",
            "send-timeout",
            "send-no-id",
            "send-changed",
            "receive",
            "receive-unknown",
            "refuse",
            "cancel",
            "list",
            "list-truncated",
            "list-failed",
            "list-empty",
            "target-denied",
            "send-denied",
            "read-denied",
            "media-denied",
            "raw-receive-read-denied",
            "raw-receive-media-denied",
            "raw-target",
            "raw-list",
        ] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path().join("state");
            let native_root = root.clone();
            let read = matches!(
                mode,
                "list"
                    | "list-truncated"
                    | "list-failed"
                    | "list-empty"
                    | "read-denied"
                    | "raw-list"
            );
            let blocked = matches!(
                mode,
                "target-denied"
                    | "send-denied"
                    | "read-denied"
                    | "media-denied"
                    | "raw-receive-read-denied"
                    | "raw-receive-media-denied"
                    | "raw-target"
            );
            let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let observed = called.clone();
            let (endpoint, server) = http::serve(
                if blocked { 0 } else { 7 },
                move |action, payload| {
                    let data = match action {
                        "get_status" => json!({"online":true}),
                        "get_login_info" => {
                            json!({"user_id":if mode=="send-changed"&&observed.load(std::sync::atomic::Ordering::SeqCst){"10003"}else{"10001"}})
                        }
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        _ => {
                            observed.store(true, std::sync::atomic::Ordering::SeqCst);
                            assert_eq!(payload["user_id"], "10002");
                            match action {
                                "send_online_file" | "send_online_folder" => {
                                    let path = std::path::Path::new(
                                        payload[if mode == "folder" {
                                            "folder_path"
                                        } else {
                                            "file_path"
                                        }]
                                        .as_str()
                                        .unwrap(),
                                    );
                                    assert!(path.canonicalize().unwrap().starts_with(
                                        native_root.join("attachments").canonicalize().unwrap()
                                    ));
                                    assert_eq!(
                                        std::fs::read(if mode == "folder" {
                                            path.join("nested/data")
                                        } else {
                                            path.to_path_buf()
                                        })
                                        .unwrap(),
                                        b"synthetic-online-input"
                                    );
                                    let pointer: Value = serde_json::from_slice(
                                        &std::fs::read(
                                            path.parent().unwrap().join(".qq-cli-attachment.json"),
                                        )
                                        .unwrap(),
                                    )
                                    .unwrap();
                                    let refs = References::existing(
                                        &Store::new(native_root.clone()),
                                        &SystemCredentials::new(&native_root).unwrap(),
                                    )
                                    .unwrap();
                                    let own: Value = refs
                                        .get("a1", pointer["reference"].as_str().unwrap())
                                        .unwrap();
                                    assert_eq!(own["persistent"], true);
                                    assert!(payload.get("file").is_none());
                                    if mode == "send-timeout" {
                                        json!({"result":-1,"errMsg":"native-secret"})
                                    } else if mode == "send-no-id" {
                                        json!({"result":0})
                                    } else {
                                        json!({"result":0,"msgId":"native-returned-msg","elementId":"native-returned-element"})
                                    }
                                }
                                "receive_online_file" | "refuse_online_file" => {
                                    assert_eq!(payload["msg_id"], "native-msg");
                                    assert_eq!(payload["element_id"], "native-element");
                                    assert!(payload.get("save_path").is_none());
                                    if mode == "receive-unknown" {
                                        json!({})
                                    } else if action == "receive_online_file" {
                                        json!({"result":0})
                                    } else {
                                        Value::Null
                                    }
                                }
                                "cancel_online_file" => {
                                    assert_eq!(payload["msg_id"], "native-msg");
                                    assert!(payload.get("element_id").is_none());
                                    Value::Null
                                }
                                "get_online_file_msg" => {
                                    if mode == "list-empty" {
                                        json!({"result":0,"msgList":[]})
                                    } else {
                                        json!({"result":if mode=="list-failed"{7}else{0},"msgList":[{"msgId":"native-msg","msgTime":"1700000000","peerUid":"native-secret","elements":[{"elementType":23,"elementId":"native-element","fileElement":{"fileName":"report.txt","fileSize":"23","filePath":"native-secret"}},{"elementType":30,"elementId":"native-folder","fileElement":{"fileName":"folder","fileSize":"","filePath":"native-secret"}},{"elementType":1,"elementId":"ignored"}]}]})
                                    }
                                }
                                _ => panic!("{action}"),
                            }
                        }
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let (store, _vault, _owned) = setup(&t, endpoint);
            let config = t.path().join("config.json");
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[if mode=="target-denied"||mode=="raw-target"{"private:10003"}else{"private:10002"}],"send":mode!="send-denied","read":mode!="read-denied"&&mode!="raw-receive-read-denied","mediaDownload":mode!="media-denied"&&mode!="raw-receive-media-denied"}})).unwrap()).unwrap();
            let file = t.path().join("input.bin");
            std::fs::write(&file, b"synthetic-online-input").unwrap();
            let dir = t.path().join("input-dir");
            std::fs::create_dir_all(dir.join("nested")).unwrap();
            std::fs::write(dir.join("nested/data"), b"synthetic-online-input").unwrap();
            let params = t.path().join("params.json");
            std::fs::write(&params,br#"{"user_id":"10002","group_id":"20001","msg_id":"native-msg","element_id":"native-element"}"#).unwrap();
            let (path, mut args) = match mode {
                "raw-list" => (
                    "api.call",
                    vec![
                        "api",
                        "call",
                        "get_online_file_msg",
                        "--params-file",
                        params.to_str().unwrap(),
                    ],
                ),
                "raw-target" | "raw-receive-read-denied" | "raw-receive-media-denied" => (
                    "api.call",
                    vec![
                        "api",
                        "call",
                        "receive_online_file",
                        "--params-file",
                        params.to_str().unwrap(),
                    ],
                ),
                "list" | "list-truncated" | "list-failed" | "list-empty" | "read-denied" => (
                    "online.list",
                    vec![
                        "online",
                        "list",
                        "10002",
                        "--limit",
                        if mode == "list-truncated" { "1" } else { "2" },
                    ],
                ),
                "folder" => (
                    "online.send-folder",
                    vec!["online", "send-folder", "10002", dir.to_str().unwrap()],
                ),
                "receive" | "receive-unknown" | "media-denied" => (
                    "online.receive",
                    vec!["online", "receive", "10002", "native-msg", "native-element"],
                ),
                "refuse" => (
                    "online.refuse",
                    vec!["online", "refuse", "10002", "native-msg", "native-element"],
                ),
                "cancel" => (
                    "online.cancel",
                    vec!["online", "cancel", "10002", "native-msg"],
                ),
                _ => (
                    "online.send",
                    vec!["online", "send", "10002", file.to_str().unwrap()],
                ),
            };
            if !read {
                args.extend(["--request-id", "online-example"]);
            }
            let server = server.start();
            let (code, value) = run(&args, &config);
            let expected = if blocked || mode == "list-failed" {
                4
            } else if matches!(
                mode,
                "send-timeout" | "send-no-id" | "send-changed" | "receive-unknown"
            ) {
                5
            } else {
                0
            };
            assert_eq!(code, expected, "{mode}: {value}");
            server.join().unwrap();
            assert_eq!(called.load(std::sync::atomic::Ordering::SeqCst), !blocked);
            assert!(
                jsonschema::validator_for(
                    &qq_cli::cli::registry::get(path).unwrap()["responseSchema"]
                )
                .unwrap()
                .is_valid(&value),
                "{mode}: {value}"
            );
            if !mode.starts_with("raw-") {
                assert!(!value.to_string().contains("native-secret"));
            }
            if mode == "list-truncated" {
                assert_eq!(value["data"]["truncated"], true);
                assert_eq!(value["data"]["complete"], false);
            }
            if !read && !blocked {
                let (cached, v) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {v}");
                if cached == 0 {
                    assert_eq!(v["data"]["deduplicated"], true);
                    assert_eq!(v["data"]["resultAvailable"], false);
                    assert!(v["data"].get("nativeMsgId").is_none());
                }
                let record = qq_cli::requests::ledger::get(&store, "online-example").unwrap();
                let text = serde_json::to_string(&record).unwrap();
                for forbidden in [
                    "synthetic-online-input",
                    "native-returned-msg",
                    "input.bin",
                    "native-secret",
                ] {
                    assert!(!text.contains(forbidden));
                }
                if matches!(
                    mode,
                    "send" | "folder" | "send-timeout" | "send-no-id" | "send-changed"
                ) {
                    assert_eq!(
                        std::fs::read_dir(store.root().join("attachments"))
                            .unwrap()
                            .count(),
                        1
                    );
                    std::fs::write(
                        if mode == "folder" {
                            dir.join("nested/data")
                        } else {
                            file.clone()
                        },
                        b"changed",
                    )
                    .unwrap();
                    assert_eq!(run(&args, &config).0, 4);
                }
            }
        }
    }
}

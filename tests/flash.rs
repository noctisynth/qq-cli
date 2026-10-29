use serde_json::{Value, json};
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
fn flash_inputs_are_explicit_and_previews_leave_no_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    let dir = t.path().join("input");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("data"), b"synthetic-flash-input").unwrap();
    for (path, args) in [
        (
            "flash.create",
            vec![
                "flash",
                "create",
                dir.to_str().unwrap(),
                "--validity-days",
                "90",
                "--dry-run",
            ],
        ),
        (
            "flash.send",
            vec!["flash", "send", "set-example", "private:10002", "--dry-run"],
        ),
        (
            "flash.download",
            vec!["flash", "download", "set-example", "--dry-run"],
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
        vec!["flash", "create", dir.to_str().unwrap()],
        vec![
            "flash",
            "create",
            dir.to_str().unwrap(),
            "--validity-days",
            "15",
            "--dry-run",
        ],
        vec!["flash", "url", "set-example"],
        vec!["flash", "url", "set-example", "--name", "x", "--index", "0"],
    ] {
        assert_eq!(run(&args, &config).0, 2, "{args:?}");
    }
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 1);
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
            let _ = self.0.delete("synthetic-flash");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn setup(t: &tempfile::TempDir, endpoint: String) -> (Store, Vault, managed::Managed) {
        let store = Store::new(t.path().join("state"));
        let vault = Vault(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-flash", "synthetic-cli-token")
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
            credential_key: "synthetic-flash".into(),
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
            "owned::flash_tasks_preserve_native_results_permissions_and_async_inputs",
        );
        (store, vault, owner)
    }

    #[test]
    fn flash_tasks_preserve_native_results_permissions_and_async_inputs() {
        if managed::child() {
            return;
        }
        for mode in [
            "create",
            "create-dir",
            "create-failed",
            "create-unknown",
            "send-private",
            "send-group",
            "send-failed",
            "send-unknown",
            "download",
            "download-denied",
            "read-denied",
            "target-denied",
            "send-denied",
            "url-denied",
            "info",
            "info-wrong",
            "list",
            "list-wrong",
            "link",
            "url",
            "resolve",
            "raw-priority",
        ] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path().join("state");
            let native_root = root.clone();
            let blocked = matches!(
                mode,
                "download-denied" | "read-denied" | "target-denied" | "send-denied" | "url-denied"
            );
            let read = matches!(
                mode,
                "info"
                    | "info-wrong"
                    | "list"
                    | "list-wrong"
                    | "link"
                    | "url"
                    | "resolve"
                    | "read-denied"
                    | "target-denied"
                    | "url-denied"
            );
            let (endpoint, server) = http::serve(
                if blocked { 0 } else { 7 },
                move |action, payload| {
                    let data = match action {
                        "get_status" => json!({"online":true}),
                        "get_login_info" => json!({"user_id":"10001"}),
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        "create_flash_task" => {
                            assert_eq!(payload["validitySeconds"], 1209600.0);
                            let paths = payload["files"].as_array().unwrap();
                            assert_eq!(paths.len(), 1);
                            let path = std::path::Path::new(paths[0].as_str().unwrap());
                            assert!(path.canonicalize().unwrap().starts_with(
                                native_root.join("attachments").canonicalize().unwrap()
                            ));
                            assert_eq!(
                                std::fs::read(if mode == "create-dir" {
                                    path.join("nested/data")
                                } else {
                                    path.to_path_buf()
                                })
                                .unwrap(),
                                b"synthetic-flash-input"
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
                            if mode == "create-unknown" {
                                json!({"task_id":"misleading-example"})
                            } else {
                                json!({"result":if mode=="create-failed"{7}else{0},"createFlashTransferResult":{"fileSetId":"set-created","shareLink":"https://invalid.invalid/share","expireTime":"1700000000"}})
                            }
                        }
                        "send_flash_msg" => {
                            if mode == "send-group" || mode == "raw-priority" {
                                assert_eq!(payload["group_id"], "20001");
                            } else {
                                assert_eq!(payload["user_id"], "10002");
                            }
                            if mode == "send-unknown" {
                                json!({"errCode":0,"rsp":{}})
                            } else {
                                json!({"errCode":0,"rsp":{"sendStatus":[{"result":if mode=="send-failed"{7}else{0}}]}})
                            }
                        }
                        "download_fileset" => {
                            json!({"result":0,"extraInfo":{"localPath":"native-secret"}})
                        }
                        "get_fileset_info" => {
                            json!({"result":0,"fileSet":{"fileSetId":if mode=="info-wrong"{"other"}else{"set-example"},"name":"set-name","totalFileCount":2,"totalFileSize":123,"status":1,"uploadStatus":2,"downloadStatus":0,"uploadInfo":{"successCount":1,"failedCount":0},"downloadInfo":{"successCount":0,"failedCount":0},"saveFileSetDir":"native-secret"}})
                        }
                        "get_flash_file_list" => {
                            json!({"result":0,"fileLists":[{"fileList":[{"fileSetId":if mode=="list-wrong"{"other"}else{"set-example"},"cliFileId":"client-example","name":"entry","isDir":false,"fileSize":"23","cliFileIndex":42,"uploadStatus":3,"downloadStatus":0,"saveFilePath":"native-secret"}]}]})
                        }
                        "get_share_link" => {
                            json!({"result":0,"shareLink":"https://invalid.invalid/share","expireTimestamp":"1700000000"})
                        }
                        "get_flash_file_url" => {
                            assert_eq!(payload["file_index"], 0.0);
                            json!({"result":0,"transferUrl":"https://invalid.invalid/file"})
                        }
                        "get_fileset_id" => json!({"fileset_id":"set-example"}),
                        _ => panic!("{action}"),
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let (store, _vault, _owned) = setup(&t, endpoint);
            let config = t.path().join("config.json");
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"read":mode!="read-denied","send":mode!="send-denied","mediaDownload":mode!="download-denied"&&mode!="url-denied","targets":if matches!(mode,"target-denied"|"raw-priority"){vec!["group:20001"]}else{vec![]}}})).unwrap()).unwrap();
            let source = t.path().join("input.bin");
            std::fs::write(&source, b"synthetic-flash-input").unwrap();
            let dir = t.path().join("input-dir");
            std::fs::create_dir_all(dir.join("nested")).unwrap();
            std::fs::write(dir.join("nested/data"), b"synthetic-flash-input").unwrap();
            let raw = t.path().join("params.json");
            std::fs::write(
                &raw,
                br#"{"fileset_id":"set-example","user_id":"10003","group_id":"20001"}"#,
            )
            .unwrap();
            let (path, mut args) = match mode {
                "info" | "info-wrong" | "read-denied" | "target-denied" => {
                    ("flash.info", vec!["flash", "info", "set-example"])
                }
                "list" | "list-wrong" => (
                    "flash.list",
                    vec!["flash", "list", "set-example", "--limit", "1"],
                ),
                "link" => ("flash.link", vec!["flash", "link", "set-example"]),
                "url" | "url-denied" => (
                    "flash.url",
                    vec!["flash", "url", "set-example", "--index", "0"],
                ),
                "resolve" => (
                    "flash.resolve",
                    vec!["flash", "resolve", "synthetic-share-code"],
                ),
                "download" | "download-denied" => {
                    ("flash.download", vec!["flash", "download", "set-example"])
                }
                "send-private" | "send-failed" | "send-unknown" | "send-denied" => (
                    "flash.send",
                    vec!["flash", "send", "set-example", "private:10002"],
                ),
                "send-group" => (
                    "flash.send",
                    vec!["flash", "send", "set-example", "group:20001"],
                ),
                "raw-priority" => (
                    "api.call",
                    vec![
                        "api",
                        "call",
                        "send_flash_msg",
                        "--params-file",
                        raw.to_str().unwrap(),
                    ],
                ),
                _ => (
                    "flash.create",
                    vec![
                        "flash",
                        "create",
                        if mode == "create-dir" {
                            dir.to_str().unwrap()
                        } else {
                            source.to_str().unwrap()
                        },
                    ],
                ),
            };
            if !read {
                args.extend(["--request-id", "flash-example"]);
            }
            let server = server.start();
            let (code, value) = run(&args, &config);
            let expected = if blocked || matches!(mode, "create-failed" | "send-failed") {
                4
            } else if matches!(mode, "create-unknown" | "send-unknown") {
                5
            } else if matches!(mode, "info-wrong" | "list-wrong") {
                3
            } else {
                0
            };
            assert_eq!(code, expected, "{mode}: {value}");
            server.join().unwrap();
            assert!(
                jsonschema::validator_for(
                    &qq_cli::cli::registry::get(path).unwrap()["responseSchema"]
                )
                .unwrap()
                .is_valid(&value),
                "{mode}: {value}"
            );
            assert!(!value.to_string().contains("native-secret"));
            if !read && !blocked {
                let (cached, v) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {v}");
                if code == 0 {
                    assert_eq!(v["data"]["deduplicated"], true);
                    assert_eq!(v["data"]["resultAvailable"], false);
                }
                let record = qq_cli::requests::ledger::get(&store, "flash-example").unwrap();
                let text = serde_json::to_string(&record).unwrap();
                for forbidden in [
                    "set-created",
                    "synthetic-flash-input",
                    "input-dir",
                    "input.bin",
                    "https://invalid",
                ] {
                    assert!(!text.contains(forbidden));
                }
                if mode.starts_with("create") {
                    assert_eq!(
                        std::fs::read_dir(store.root().join("attachments"))
                            .unwrap()
                            .count(),
                        1
                    );
                    std::fs::write(
                        if mode == "create-dir" {
                            dir.join("nested/data")
                        } else {
                            source.clone()
                        },
                        b"different",
                    )
                    .unwrap();
                    assert_eq!(run(&args, &config).0, 4);
                }
            }
        }
    }
}

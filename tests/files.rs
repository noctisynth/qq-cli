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
fn previews_validate_explicit_inputs_and_packet_precision_without_state() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    let file = t.path().join("report.txt");
    std::fs::write(&file, b"synthetic-source").unwrap();
    for args in [
        vec!["file", "mkdir", "20001", "name"],
        vec![
            "file",
            "upload",
            "private:10002",
            file.to_str().unwrap(),
            "--folder",
            "folder-example",
            "--dry-run",
        ],
        vec![
            "file",
            "move",
            "9007199254740993",
            "alias",
            "--from",
            "/",
            "--to",
            "target",
            "--dry-run",
        ],
    ] {
        assert_eq!(run(&args, &config).0, 2, "{args:?}");
    }
    for (path, args) in [
        (
            "file.mkdir",
            vec!["file", "mkdir", "20001", "name", "--dry-run"],
        ),
        (
            "file.rmdir",
            vec!["file", "rmdir", "20001", "folder-example", "--dry-run"],
        ),
        (
            "file.remove",
            vec!["file", "remove", "20001", "alias", "--dry-run"],
        ),
        (
            "file.move",
            vec![
                "file",
                "move",
                "20001",
                "alias",
                "--from",
                "/",
                "--to",
                "target",
                "--dry-run",
            ],
        ),
        (
            "file.rename",
            vec![
                "file",
                "rename",
                "20001",
                "alias",
                "new-name",
                "--folder",
                "/",
                "--dry-run",
            ],
        ),
        (
            "file.transfer",
            vec!["file", "transfer", "20001", "alias", "--dry-run"],
        ),
        (
            "file.upload",
            vec![
                "file",
                "upload",
                "group:20001",
                file.to_str().unwrap(),
                "--folder",
                "folder-example",
                "--dry-run",
            ],
        ),
        (
            "file.upload",
            vec![
                "file",
                "upload",
                "private:10002",
                file.to_str().unwrap(),
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
        assert!(!value.to_string().contains("report.txt"));
    }
    let (_, v) = run(&["api", "describe", "download_file"], &config);
    assert_eq!(v["data"]["effect"], "write");
    assert_eq!(v["data"]["parametersKnown"], true);
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
            let _ = self.0.delete("synthetic-files");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn setup(t: &tempfile::TempDir, endpoint: String) -> (Store, Vault, managed::Managed) {
        let store = Store::new(t.path().join("state"));
        let vault = Vault(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-files", "synthetic-cli-token")
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
            credential_key: "synthetic-files".into(),
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
            "owned::managed_file_operations_use_actual_groups_owned_sources_and_once_only_keys",
        );
        (store, vault, owner)
    }
    fn source_message() -> Value {
        json!({"message_id":"40000","real_seq":"100","time":1700000000,"self_id":"10001","user_id":"10002","message_type":"private","target_id":"10001","message":[{"type":"file","data":{"file":"sample.bin","file_id":"opaque-file-uuid","file_size":"3"}}]})
    }
    #[test]
    fn managed_file_operations_use_actual_groups_owned_sources_and_once_only_keys() {
        if managed::child() {
            return;
        }
        for mode in [
            "info",
            "root",
            "folder",
            "truncated",
            "wrong-group",
            "mkdir",
            "rmdir",
            "rmdir-failed",
            "remove",
            "remove-failed",
            "move",
            "rename",
            "transfer",
            "false-ok",
            "missing-ok",
            "upload-group",
            "upload-private",
            "upload-null",
            "upload-malformed",
            "upload-changed",
            "url",
            "url-denied",
            "target-denied",
            "send-denied",
            "private-url",
            "private-stale",
            "private-denied",
        ] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path().join("state");
            let native_root = root.clone();
            let reads = matches!(
                mode,
                "info"
                    | "root"
                    | "folder"
                    | "truncated"
                    | "wrong-group"
                    | "url"
                    | "url-denied"
                    | "private-url"
                    | "private-stale"
                    | "private-denied"
            );
            let blocked = matches!(
                mode,
                "url-denied" | "private-denied" | "target-denied" | "send-denied"
            );
            let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = calls.clone();
            let (endpoint, server) = http::serve(
                if blocked {
                    0
                } else if mode == "private-url" {
                    14
                } else {
                    7
                },
                move |action, payload| {
                    let data = match action {
                        "get_status" => json!({"online":true}),
                        "get_login_info" => {
                            json!({"user_id":if mode=="upload-changed"&&observed.load(std::sync::atomic::Ordering::SeqCst)>0{"10003"}else{"10001"}})
                        }
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        "get_msg" => {
                            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            let mut raw = source_message();
                            if mode == "private-stale" {
                                raw["time"] = json!(1700000001);
                            }
                            raw
                        }
                        "get_private_file_url" => {
                            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            assert_eq!(payload["file_id"], "opaque-file-uuid");
                            json!({"url":"https://invalid.invalid/private-file"})
                        }
                        _ => {
                            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            if mode == "upload-private" {
                                assert_eq!(payload["user_id"], "10002");
                                assert!(payload.get("group_id").is_none());
                            } else {
                                assert_eq!(payload["group_id"], "20001");
                            }
                            match mode {
                                "info" => {
                                    assert_eq!(action, "get_group_file_system_info");
                                    json!({"file_count":2,"limit_count":10000,"used_space":0,"total_space":10737418240_u64,"secret":"native-secret"})
                                }
                                "root" | "folder" | "truncated" | "wrong-group" => {
                                    assert_eq!(
                                        action,
                                        if mode == "folder" {
                                            "get_group_files_by_folder"
                                        } else {
                                            "get_group_root_files"
                                        }
                                    );
                                    assert_eq!(
                                        payload["file_count"],
                                        if mode == "truncated" { 1.0 } else { 2.0 }
                                    );
                                    if mode == "folder" {
                                        assert_eq!(payload["folder"], "folder-example");
                                    }
                                    json!({"files":[{"group_id":if mode=="wrong-group"{20002}else{20001},"file_id":"alias-example","file_name":"report.txt","file_size":17,"uploader":10002,"secret":"native-secret"}],"folders":[{"group_id":20001,"folder_id":"folder-example","folder_name":"folder","total_file_count":2}]})
                                }
                                "mkdir" => {
                                    assert_eq!(action, "create_group_file_folder");
                                    assert_eq!(payload["folder_name"], "folder-name");
                                    json!({"result":{},"groupItem":[],"secret":"native-secret"})
                                }
                                "rmdir" | "rmdir-failed" => {
                                    assert_eq!(action, "delete_group_folder");
                                    assert_eq!(payload["folder_id"], "folder-example");
                                    json!({"retCode":if mode=="rmdir-failed"{7}else{0}})
                                }
                                "remove" | "remove-failed" => {
                                    assert_eq!(action, "delete_group_file");
                                    assert_eq!(payload["file_id"], "alias-example");
                                    json!({"result":0,"transGroupFileResult":{"failFileIdList":if mode=="remove-failed"{vec!["failed"]}else{vec![]}}})
                                }
                                "move" | "false-ok" | "missing-ok" => {
                                    assert_eq!(action, "move_group_file");
                                    assert_eq!(payload["current_parent_directory"], "/");
                                    assert_eq!(payload["target_parent_directory"], "target");
                                    if mode == "missing-ok" {
                                        json!({})
                                    } else {
                                        json!({"ok":mode!="false-ok"})
                                    }
                                }
                                "rename" => {
                                    assert_eq!(action, "rename_group_file");
                                    assert_eq!(payload["new_name"], "new-name");
                                    assert_eq!(payload["current_parent_directory"], "/");
                                    json!({"ok":true})
                                }
                                "transfer" => {
                                    assert_eq!(action, "trans_group_file");
                                    json!({"ok":true})
                                }
                                "url" => {
                                    assert_eq!(action, "get_group_file_url");
                                    json!({"url":"https://invalid.invalid/native-file","secret":"native-secret"})
                                }
                                _ => {
                                    assert_eq!(
                                        action,
                                        if mode == "upload-private" {
                                            "upload_private_file"
                                        } else {
                                            "upload_group_file"
                                        }
                                    );
                                    assert_eq!(payload["upload_file"], true);
                                    assert_eq!(payload["name"], "report.txt");
                                    let path =
                                        std::path::PathBuf::from(payload["file"].as_str().unwrap());
                                    assert!(path.canonicalize().unwrap().starts_with(
                                        native_root.join("attachments").canonicalize().unwrap()
                                    ));
                                    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-source");
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
                                    let record = qq_cli::requests::ledger::get(
                                        &Store::new(native_root.clone()),
                                        "files-example",
                                    )
                                    .unwrap();
                                    assert_eq!(own["intentPayloadHash"], record.payload_hash);
                                    assert_eq!(
                                        record.outcome,
                                        qq_cli::core::contracts::write::Outcome::Pending
                                    );
                                    if mode == "upload-null" {
                                        json!({"file_id":null})
                                    } else if mode == "upload-malformed" {
                                        json!({})
                                    } else {
                                        json!({"file_id":"native-upload-uuid","secret":"native-secret"})
                                    }
                                }
                            }
                        }
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let (store, _vault, _owned) = setup(&t, endpoint);
            let media_token = if mode.starts_with("private-") {
                let mut tx = store
                    .transaction(Instant::now() + Duration::from_secs(5))
                    .unwrap();
                let mut other = tx.registry.instances[0].clone();
                other.id = "unrelated-instance".into();
                other.credential_key = "unrelated-absent-key".into();
                tx.registry.instances.push(other);
                tx.commit().unwrap();

                let refs =
                    References::open(&store, &_vault.0, Instant::now() + Duration::from_secs(5))
                        .unwrap();
                let scope = qq_cli::core::domain::Scope {
                    instance_id: "instance-example".into(),
                    account: "10001".into(),
                };
                let msg = qq_cli::messaging::message::normalize(
                    &source_message(),
                    &scope,
                    "run-example",
                    &"private:10002".parse().unwrap(),
                    &refs,
                )
                .unwrap();
                serde_json::to_value(msg.value).unwrap()["segments"][0]["mediaRef"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            } else {
                String::new()
            };
            let config = t.path().join("config.json");
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[if mode=="target-denied"{"group:20002"}else if mode=="upload-private"||mode.starts_with("private-"){"private:10002"}else{"group:20001"}],"send":mode!="send-denied","mediaDownload":mode!="url-denied"&&mode!="private-denied"}})).unwrap()).unwrap();
            let file = t.path().join("report.txt");
            std::fs::write(&file, b"synthetic-source").unwrap();
            let (path, mut args) = match mode {
                "private-url" | "private-stale" | "private-denied" => (
                    "file.private-url",
                    vec!["file", "private-url", &media_token],
                ),
                "info" => ("file.info", vec!["file", "info", "20001"]),
                "root" | "truncated" | "wrong-group" => (
                    "file.list",
                    vec![
                        "file",
                        "list",
                        "20001",
                        "--limit",
                        if mode == "truncated" { "1" } else { "2" },
                    ],
                ),
                "folder" => (
                    "file.list",
                    vec![
                        "file",
                        "list",
                        "20001",
                        "--folder",
                        "folder-example",
                        "--limit",
                        "2",
                    ],
                ),
                "mkdir" | "target-denied" | "send-denied" => {
                    ("file.mkdir", vec!["file", "mkdir", "20001", "folder-name"])
                }
                "rmdir" | "rmdir-failed" => (
                    "file.rmdir",
                    vec!["file", "rmdir", "20001", "folder-example"],
                ),
                "remove" | "remove-failed" => (
                    "file.remove",
                    vec!["file", "remove", "20001", "alias-example"],
                ),
                "move" | "false-ok" | "missing-ok" => (
                    "file.move",
                    vec![
                        "file",
                        "move",
                        "20001",
                        "alias-example",
                        "--from",
                        "/",
                        "--to",
                        "target",
                    ],
                ),
                "rename" => (
                    "file.rename",
                    vec![
                        "file",
                        "rename",
                        "20001",
                        "alias-example",
                        "new-name",
                        "--folder",
                        "/",
                    ],
                ),
                "transfer" => (
                    "file.transfer",
                    vec!["file", "transfer", "20001", "alias-example"],
                ),
                "url" | "url-denied" => ("file.url", vec!["file", "url", "20001", "alias-example"]),
                _ => (
                    "file.upload",
                    vec![
                        "file",
                        "upload",
                        if mode == "upload-private" {
                            "private:10002"
                        } else {
                            "group:20001"
                        },
                        file.to_str().unwrap(),
                    ],
                ),
            };
            if !reads {
                args.extend(["--request-id", "files-example"]);
            }
            let server = server.start();
            let (code, value) = run(&args, &config);
            let expected = if blocked
                || matches!(
                    mode,
                    "rmdir-failed" | "remove-failed" | "false-ok" | "private-stale"
                ) {
                4
            } else if matches!(mode, "upload-malformed" | "upload-changed" | "missing-ok") {
                5
            } else if mode == "wrong-group" {
                3
            } else {
                0
            };
            assert_eq!(code, expected, "{mode}: {value}");
            server.join().unwrap();
            assert_eq!(
                calls.load(std::sync::atomic::Ordering::SeqCst),
                if mode == "private-url" {
                    2
                } else {
                    usize::from(!blocked)
                }
            );
            let schema = qq_cli::cli::registry::get(path).unwrap();
            assert!(
                jsonschema::validator_for(&schema["responseSchema"])
                    .unwrap()
                    .is_valid(&value),
                "{value}"
            );
            assert!(!value.to_string().contains("native-secret"));
            if mode == "truncated" {
                assert_eq!(value["data"]["truncated"], true);
                assert_eq!(value["data"]["folders"], json!([]));
            }
            if !reads && !blocked {
                let (cached, v) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {v}");
                if cached == 0 {
                    assert_eq!(v["data"]["fileIdAvailable"], false);
                    assert_eq!(v["data"]["deduplicated"], true);
                }
                let record = qq_cli::requests::ledger::get(&store, "files-example").unwrap();
                let text = serde_json::to_string(&record).unwrap();
                for forbidden in ["synthetic-source", "report.txt", "native-upload-uuid"] {
                    assert!(!text.contains(forbidden));
                }
            }
        }
    }
}

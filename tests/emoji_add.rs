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
fn emoji_import_previews_check_only_explicit_local_input_and_numeric_package() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.png");
    std::fs::write(&path, b"synthetic-image-bytes").unwrap();
    let config = temp.path().join("missing.json");
    assert_eq!(run(&["emoji", "add", path.to_str().unwrap()], &config).0, 2);
    let (c, v) = run(
        &[
            "emoji",
            "add",
            path.to_str().unwrap(),
            "--emoji-id",
            "opaque-id",
            "--package-id",
            "0",
            "--original",
            "false",
            "--dry-run",
        ],
        &config,
    );
    assert_eq!(c, 0, "{v}");
    assert_eq!(v["data"]["identityVerified"], false);
    assert!(!temp.path().join("state").exists());
    assert_eq!(
        run(
            &[
                "emoji",
                "add",
                path.to_str().unwrap(),
                "--package-id",
                "9007199254740993",
                "--dry-run"
            ],
            &config
        )
        .0,
        2
    );
    let huge = temp.path().join("huge.png");
    std::fs::File::create(&huge)
        .unwrap()
        .set_len(128 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        run(
            &["emoji", "add", huge.to_str().unwrap(), "--dry-run"],
            &config
        )
        .0,
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
fn managed_emoji_imports_use_only_owned_snapshots_and_never_replay_after_acceptance_or_uncertainty()
{
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        references::References,
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in ["default", "advanced", "rejected", "changed", "send-denied"] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-emoji-add");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let t = tempfile::tempdir().unwrap();
        let store = Store::new(t.path().join("state"));
        let vault = Cleanup(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-emoji-add", "synthetic-cli-token")
            .unwrap();
        let image = t.path().join("image.png");
        std::fs::write(&image, b"synthetic-image-bytes").unwrap();
        let original = image.clone();
        let root = store.root().to_path_buf();
        let server_root = root.clone();
        let writes = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = writes.clone();
        let (endpoint, server) = http::serve(
            if mode == "send-denied" { 0 } else { 7 },
            move |action, payload| {
                let data = match action {
                    "get_version_info" => json!({"app_version":"4.18.38"}),
                    "get_status" => json!({"online":true}),
                    "get_login_info" => {
                        json!({"user_id":if mode=="changed"&&seen.load(std::sync::atomic::Ordering::SeqCst)>0{"10002"}else{"10001"}})
                    }
                    "add_custom_face" => {
                        seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let path = std::path::Path::new(payload["file"].as_str().unwrap());
                        assert_ne!(path, original);
                        assert!(
                            path.canonicalize().unwrap().starts_with(
                                server_root.join("attachments").canonicalize().unwrap()
                            )
                        );
                        assert_eq!(std::fs::read(path).unwrap(), b"synthetic-image-bytes");
                        assert_eq!(payload["file_name"], "image.png");
                        assert_eq!(payload["file_size"], "21");
                        assert!(payload.get("md5").is_none());
                        assert_eq!(
                            payload["package_id"],
                            if mode == "advanced" { "7" } else { "0" }
                        );
                        assert_eq!(
                            payload["emoji_id"],
                            if mode == "advanced" { "opaque-id" } else { "" }
                        );
                        assert_eq!(payload["is_mark_face"], mode == "advanced");
                        assert_eq!(payload["is_origin"], mode != "advanced");
                        let pointer: Value = serde_json::from_slice(
                            &std::fs::read(path.parent().unwrap().join(".qq-cli-attachment.json"))
                                .unwrap(),
                        )
                        .unwrap();
                        let creds = SystemCredentials::new(&server_root).unwrap();
                        let refs =
                            References::existing(&Store::new(server_root.clone()), &creds).unwrap();
                        let owned: Value = refs
                            .get("a1", pointer["reference"].as_str().unwrap())
                            .unwrap();
                        assert_eq!(owned["persistent"], true);
                        assert_eq!(
                            qq_cli::requests::ledger::get(
                                &Store::new(server_root.clone()),
                                "emoji-add-example"
                            )
                            .unwrap()
                            .outcome,
                            qq_cli::requests::ledger::Outcome::Pending
                        );
                        Value::Null
                    }
                    _ => panic!("unexpected synthetic import action"),
                };
                serde_json::to_vec(&json!({"status":if mode=="rejected"&&action=="add_custom_face"{"failed"}else{"ok"},"retcode":if mode=="rejected"&&action=="add_custom_face"{1200}else{0},"data":data})).unwrap()
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
            credential_key: "synthetic-emoji-add".into(),
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
            "managed_emoji_imports_use_only_owned_snapshots_and_never_replay_after_acceptance_or_uncertainty",
        );
        let config = t.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":["group:20001"],"read":false,"send":mode!="send-denied"}})).unwrap()).unwrap();
        let mut args = vec![
            "emoji",
            "add",
            image.to_str().unwrap(),
            "--request-id",
            "emoji-add-example",
        ];
        if mode == "advanced" {
            args.extend([
                "--emoji-id",
                "opaque-id",
                "--package-id",
                "7",
                "--market",
                "--original",
                "false",
            ]);
        }
        let server = server.start();
        let (c, v) = run(&args, &config);
        let expected = match mode {
            "rejected" | "send-denied" => 4,
            "changed" => 5,
            _ => 0,
        };
        assert_eq!(c, expected, "{mode}: {v}");
        let schema = qq_cli::cli::registry::get("emoji.add").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&v),
            "{v}"
        );
        server.join().unwrap();
        assert_eq!(
            writes.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(mode != "send-denied")
        );
        if mode != "send-denied" {
            let (cached, v) = run(&args, &config);
            assert_eq!(cached, c, "{v}");
            if c == 0 {
                assert_eq!(v["data"]["deduplicated"], true);
            }
            assert_eq!(
                std::fs::read_dir(root.join("attachments")).unwrap().count(),
                1
            );
            let (_, record) = run(
                &[
                    "request",
                    "get",
                    "emoji-add-example",
                    "--instance",
                    "instance-example",
                ],
                &config,
            );
            assert_eq!(record["data"]["action"], "add_custom_face");
            assert_eq!(record["data"]["resultAvailable"], false);
        } else {
            assert!(!root.join("attachments").exists());
        }
    }
}

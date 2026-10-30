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
fn cleanup_requires_a_key_and_offline_preview_states_the_real_scope() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    assert_eq!(run(&["napcat", "clean-cache"], &config).0, 2);
    let (code, v) = run(&["napcat", "clean-cache", "--dry-run"], &config);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["cache"], "backend_and_qq_media");
    assert_eq!(v["data"]["cleanupVerified"], false);
    assert_eq!(v["data"]["identityVerified"], false);
    assert!(
        jsonschema::validator_for(
            &qq_cli::cli::registry::get("napcat.clean-cache").unwrap()["responseSchema"]
        )
        .unwrap()
        .is_valid(&v)
    );
    let (_, v) = run(&["api", "describe", "clean_cache"], &config);
    assert_eq!(v["data"]["parametersKnown"], true);
    assert_eq!(v["data"]["effect"], "write");
    assert_eq!(v["data"]["sensitiveResult"], false);
    assert!(
        v["data"]["resultNote"]
            .as_str()
            .unwrap()
            .contains("individual failures")
    );
    let (_, v) = run(&["api", "describe", "bot_exit"], &config);
    assert_eq!(v["data"]["parametersKnown"], false);
    let (_, v) = run(&["api", "describe", "set_restart"], &config);
    assert_eq!(v["data"]["parametersKnown"], false);
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 0);
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
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    struct Vault(SystemCredentials);
    impl Drop for Vault {
        fn drop(&mut self) {
            let _ = self.0.delete("synthetic-cleanup");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn setup(t: &tempfile::TempDir, endpoint: String) -> (Store, Vault, managed::Managed) {
        let store = Store::new(t.path().join("state"));
        let vault = Vault(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-cleanup", "synthetic-cli-token")
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
            credential_key: "synthetic-cleanup".into(),
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
            "owned::native_cleanup_is_explicit_once_only_and_never_claims_all_files_removed",
        );
        (store, vault, owner)
    }

    #[test]
    fn native_cleanup_is_explicit_once_only_and_never_claims_all_files_removed() {
        if managed::child() {
            return;
        }
        for mode in [
            "ok",
            "missing-data",
            "native-failed",
            "native-async",
            "native-malformed",
            "identity-changed",
            "preflight",
            "send-denied",
            "target-denied",
            "raw-first",
            "raw-suffix",
            "raw-target",
        ] {
            let t = tempfile::tempdir().unwrap();
            let blocked = matches!(mode, "send-denied" | "target-denied" | "raw-target");
            let called = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = called.clone();
            let (endpoint, server) = http::serve(
                if blocked {
                    0
                } else if mode == "preflight" {
                    3
                } else {
                    7
                },
                move |action, payload| {
                    let data = match action {
                        "get_status" => json!({"online":true}),
                        "get_login_info" => {
                            json!({"user_id":if mode=="preflight"||(mode=="identity-changed"&&observed.load(std::sync::atomic::Ordering::SeqCst)>0){"10003"}else{"10001"}})
                        }
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        "clean_cache" | "clean_cache_async" => {
                            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            assert_eq!(payload, &json!({}));
                            if mode == "missing-data" {
                                return serde_json::to_vec(&json!({"status":"ok","retcode":0}))
                                    .unwrap();
                            }
                            if matches!(mode, "native-failed" | "native-async" | "raw-suffix") {
                                return serde_json::to_vec(&json!({"status":if mode=="native-failed"{"failed"}else{"async"},"retcode":if mode=="native-failed"{71}else{0},"data":null,"message":"synthetic-native-private-error"})).unwrap();
                            }
                            if mode == "native-malformed" {
                                json!({"deleted":100,"path":"native-untrusted"})
                            } else {
                                Value::Null
                            }
                        }
                        _ => panic!("{action}"),
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let (store, _vault, _owned) = setup(&t, endpoint);
            let config = t.path().join("config.json");
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"read":false,"send":mode!="send-denied","targets":if matches!(mode,"target-denied"|"raw-target"){vec!["group:20001"]}else{vec![]}}})).unwrap()).unwrap();
            let untouched = t.path().join("synthetic-local-cache.bin");
            std::fs::write(&untouched, b"untouched-local-content").unwrap();
            let params = t.path().join("params.json");
            std::fs::write(&params, br#"{"group_id":"20001","user_id":"10002"}"#).unwrap();
            let raw = mode.starts_with("raw-");
            let args = if raw {
                let mut v = vec![
                    "api",
                    "call",
                    if mode == "raw-suffix" {
                        "clean_cache_async"
                    } else {
                        "clean_cache"
                    },
                    "--request-id",
                    "cleanup-example",
                ];
                if mode == "raw-target" {
                    v.extend(["--params-file", params.to_str().unwrap()]);
                }
                v
            } else {
                vec!["napcat", "clean-cache", "--request-id", "cleanup-example"]
            };
            let server = server.start();
            let (code, v) = run(&args, &config);
            let expected = if blocked || mode == "preflight" {
                4
            } else if matches!(
                mode,
                "native-failed"
                    | "native-async"
                    | "native-malformed"
                    | "identity-changed"
                    | "raw-suffix"
            ) {
                5
            } else {
                0
            };
            assert_eq!(code, expected, "{mode}: {v}");
            server.join().unwrap();
            assert_eq!(
                called.load(std::sync::atomic::Ordering::SeqCst),
                usize::from(!blocked && mode != "preflight")
            );
            assert!(
                jsonschema::validator_for(
                    &qq_cli::cli::registry::get(if raw {
                        "api.call"
                    } else {
                        "napcat.clean-cache"
                    })
                    .unwrap()["responseSchema"]
                )
                .unwrap()
                .is_valid(&v),
                "{mode}: {v}"
            );
            if !raw {
                assert!(!v.to_string().contains("synthetic-native-private-error"));
            }
            assert_eq!(
                std::fs::read(&untouched).unwrap(),
                b"untouched-local-content"
            );
            if !blocked {
                let (cached, reply) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {reply}");
                if cached == 0 && !raw {
                    assert_eq!(reply["data"]["deduplicated"], true);
                    assert_eq!(reply["data"]["cleanupVerified"], false);
                    assert_eq!(v["data"]["cleanupVerified"], false);
                    assert!(v["data"].get("removedFiles").is_none());
                }
                let record = qq_cli::requests::ledger::get(&store, "cleanup-example").unwrap();
                assert!(
                    !serde_json::to_string(&record)
                        .unwrap()
                        .contains("synthetic-native-private-error")
                );
                if mode == "raw-first" {
                    let (cached, reply) = run(
                        &["napcat", "clean-cache", "--request-id", "cleanup-example"],
                        &config,
                    );
                    assert_eq!(cached, 0, "{reply}");
                    assert_eq!(reply["data"]["deduplicated"], true);
                    assert_eq!(reply["data"]["cleanupVerified"], false);
                }
                if mode == "ok" {
                    let (cached, reply) = run(
                        &[
                            "api",
                            "call",
                            "clean_cache",
                            "--request-id",
                            "cleanup-example",
                        ],
                        &config,
                    );
                    assert_eq!(cached, 0, "{reply}");
                    assert_eq!(reply["data"]["deduplicated"], true);
                    assert_eq!(reply["data"]["resultAvailable"], false);
                }
            }
        }
    }
}

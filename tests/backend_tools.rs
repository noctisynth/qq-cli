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
fn reviewed_tool_metadata_exposes_wire_payloads_secrets_and_native_limitations_offline() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    for action in [
        "get_cookies",
        "get_csrf_token",
        "get_credentials",
        "get_clientkey",
        "get_rkey",
        "get_rkey_server",
        "nc_get_rkey",
        "nc_get_packet_status",
        "send_packet",
        "get_robot_uin_range",
        "check_url_safely",
    ] {
        let (code, v) = run(&["api", "describe", action], &config);
        assert_eq!(code, 0);
        assert_eq!(v["data"]["parametersKnown"], true, "{action}");
        assert_eq!(
            v["data"]["effect"],
            if action == "send_packet" {
                "write"
            } else {
                "read"
            }
        );
        assert_eq!(v["data"]["requestIdRequired"], action == "send_packet");
        assert_eq!(
            v["data"]["sensitiveResult"],
            matches!(
                action,
                "get_cookies"
                    | "get_csrf_token"
                    | "get_credentials"
                    | "get_clientkey"
                    | "get_rkey"
                    | "get_rkey_server"
                    | "nc_get_rkey"
            )
        );
        assert!(
            v["data"]["resultNote"]
                .as_str()
                .is_some_and(|s| !s.is_empty())
        );
        let payload = match action {
            "get_cookies" | "get_credentials" => {
                serde_json::to_value(qq_cli::core::contracts::backend_tools::DomainPayload {
                    domain: "qun.qq.com".into(),
                })
                .unwrap()
            }
            "check_url_safely" => {
                serde_json::to_value(qq_cli::core::contracts::backend_tools::UrlPayload {
                    url: "https://invalid.invalid/synthetic".into(),
                })
                .unwrap()
            }
            "send_packet" => {
                serde_json::to_value(qq_cli::core::contracts::backend_tools::PacketPayload {
                    cmd: "Synthetic.Cmd".into(),
                    data: "0102".into(),
                    rsp: qq_cli::core::contracts::backend_tools::Response::Text("false".into()),
                })
                .unwrap()
            }
            _ => serde_json::to_value(qq_cli::core::contracts::backend_tools::EmptyPayload {})
                .unwrap(),
        };
        let validator = jsonschema::validator_for(&v["data"]["parameterSchema"]).unwrap();
        assert!(validator.is_valid(&payload), "{action}: {payload}");
        if matches!(
            action,
            "get_cookies" | "get_credentials" | "check_url_safely" | "send_packet"
        ) {
            assert!(!validator.is_valid(&json!({})), "{action}");
        }
        let schema = qq_cli::cli::registry::get("api.describe").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&v)
        );
    }
    let (_, v) = run(&["api", "describe", "check_url_safely"], &config);
    assert_eq!(v["data"]["compatibilityOnly"], true);
    let (_, v) = run(&["api", "describe", "future_tool"], &config);
    assert_eq!(v["data"]["sensitiveResult"], false);
    assert_eq!(v["data"]["effect"], "unknown");
    assert_eq!(run(&["api", "call", "send_packet"], &config).0, 2);
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
            let _ = self.0.delete("synthetic-tools");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn setup(t: &tempfile::TempDir, endpoint: String) -> (Store, Vault, managed::Managed) {
        let store = Store::new(t.path().join("state"));
        let vault = Vault(SystemCredentials::new(store.root()).unwrap());
        vault
            .0
            .set("synthetic-tools", "synthetic-cli-token")
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
            credential_key: "synthetic-tools".into(),
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
            "owned::tools_preserve_native_results_permissions_and_secret_free_keys",
        );
        (store, vault, owner)
    }

    #[test]
    fn tools_preserve_native_results_permissions_and_secret_free_keys() {
        if managed::child() {
            return;
        }
        for mode in [
            "cookies",
            "csrf",
            "credentials",
            "clientkey",
            "rkey",
            "rkey-server",
            "nc-rkey",
            "packet-status",
            "ranges",
            "url",
            "packet",
            "packet-failed",
            "packet-no-read",
            "packet-no-send",
            "packet-target",
            "credential-target",
            "credential-read-denied",
            "cookie-keyed",
            "cookie-changed",
            "cookie-failed",
            "packet-async",
        ] {
            let t = tempfile::tempdir().unwrap();
            let blocked = matches!(
                mode,
                "packet-no-read"
                    | "packet-no-send"
                    | "packet-target"
                    | "credential-target"
                    | "credential-read-denied"
            );
            let write = mode.starts_with("packet") && mode != "packet-status";
            let keyed = write || mode.starts_with("cookie-");
            let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let observed = called.clone();
            let action = match mode {
                "csrf" => "get_csrf_token",
                "credentials" => "get_credentials",
                "clientkey" => "get_clientkey",
                "rkey" => "get_rkey",
                "rkey-server" => "get_rkey_server",
                "nc-rkey" => "nc_get_rkey",
                "packet-status" => "nc_get_packet_status",
                "ranges" => "get_robot_uin_range",
                "url" => "check_url_safely",
                _ if write => "send_packet",
                _ => "get_cookies",
            };
            let (endpoint, server) = http::serve(
                if blocked { 0 } else { 7 },
                move |native, payload| {
                    let data = match native {
                        "get_status" => json!({"online":true}),
                        "get_login_info" => {
                            json!({"user_id":if mode=="cookie-changed"&&observed.load(std::sync::atomic::Ordering::SeqCst){"10003"}else{"10001"}})
                        }
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        _ => {
                            assert_eq!(native, action);
                            observed.store(true, std::sync::atomic::Ordering::SeqCst);
                            let expected = match native {
                                "get_cookies" => {
                                    assert_eq!(payload["domain"], "qun.qq.com");
                                    json!({"cookies":"synthetic-tool-secret","bkn":"123"})
                                }
                                "get_credentials" => {
                                    json!({"cookies":"synthetic-tool-secret","token":123})
                                }
                                "get_csrf_token" => json!({"token":123}),
                                "get_clientkey" => json!({"clientkey":"synthetic-tool-secret"}),
                                "get_rkey" => {
                                    json!([{"type":"private","rkey":"synthetic-tool-secret","created_at":1700000000,"ttl":3600}])
                                }
                                "get_rkey_server" => {
                                    json!({"private_rkey":"synthetic-tool-secret","name":"NapCat 4"})
                                }
                                "nc_get_rkey" => {
                                    json!([{"unreviewed":{"key":"synthetic-tool-secret"}}])
                                }
                                "nc_get_packet_status" => Value::Null,
                                "get_robot_uin_range" => {
                                    json!([{"unreviewedRange":["10000","20000"],"extra":false}])
                                }
                                "check_url_safely" => {
                                    assert_eq!(payload["url"], "https://invalid.invalid/synthetic");
                                    json!({"level":1})
                                }
                                "send_packet" => {
                                    assert_eq!(payload["cmd"], "Synthetic.Cmd");
                                    assert_eq!(payload["data"], "0102");
                                    assert_eq!(payload["rsp"], "false");
                                    assert_eq!(payload["future"], json!({"preserve":true}));
                                    json!("synthetic-tool-secret")
                                }
                                _ => panic!("{native}"),
                            };
                            if matches!(mode, "packet-failed" | "cookie-failed" | "packet-async") {
                                return serde_json::to_vec(&json!({"status":if mode=="packet-async"{"async"}else{"failed"},"retcode":if mode=="packet-async"{0}else{71},"data":expected})).unwrap();
                            }
                            expected
                        }
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let (store, _vault, _owned) = setup(&t, endpoint);
            let config = t.path().join("config.json");
            let restricted = matches!(
                mode,
                "packet-target" | "credential-target" | "packet-status" | "ranges" | "url"
            );
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"send":write&&mode!="packet-no-send","read":mode!="packet-no-read"&&mode!="credential-read-denied","targets":if restricted{vec!["group:20001"]}else{vec![]}}})).unwrap()).unwrap();
            let params = t.path().join("params.json");
            let mut payload = match action {
                "get_cookies" | "get_credentials" => json!({"domain":"qun.qq.com"}),
                "check_url_safely" => json!({"url":"https://invalid.invalid/synthetic"}),
                "send_packet" => {
                    json!({"cmd":"Synthetic.Cmd","data":"0102","rsp":"false","future":{"preserve":true}})
                }
                _ => json!({}),
            };
            if restricted {
                payload["group_id"] = json!("20001");
                payload["user_id"] = json!("10002");
            }
            std::fs::write(&params, serde_json::to_vec(&payload).unwrap()).unwrap();
            let mut args = vec![
                "api",
                "call",
                action,
                "--params-file",
                params.to_str().unwrap(),
            ];
            if keyed {
                args.extend(["--request-id", "tools-example"]);
            }
            let server = server.start();
            let (code, v) = run(&args, &config);
            let expected = if blocked || mode == "cookie-failed" {
                4
            } else if matches!(mode, "packet-failed" | "packet-async" | "cookie-changed") {
                5
            } else {
                0
            };
            assert_eq!(code, expected, "{mode}: {v}");
            server.join().unwrap();
            assert_eq!(called.load(std::sync::atomic::Ordering::SeqCst), !blocked);
            assert!(
                jsonschema::validator_for(
                    &qq_cli::cli::registry::get("api.call").unwrap()["responseSchema"]
                )
                .unwrap()
                .is_valid(&v),
                "{mode}: {v}"
            );
            if mode == "url" {
                assert_eq!(v["data"]["compatibilityOnly"], true);
            }
            if mode == "packet-status" {
                assert!(v["data"]["result"]["data"].is_null());
            }
            if mode == "nc-rkey" {
                assert_eq!(
                    v["data"]["result"]["data"][0]["unreviewed"]["key"],
                    "synthetic-tool-secret"
                );
            }
            if keyed && !blocked {
                let (cached, reply) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {reply}");
                assert_eq!(reply["data"]["resultAvailable"], false);
                let record = qq_cli::requests::ledger::get(&store, "tools-example").unwrap();
                let text = serde_json::to_string(&record).unwrap();
                for secret in [
                    "synthetic-tool-secret",
                    "qun.qq.com",
                    "Synthetic.Cmd",
                    "0102",
                ] {
                    assert!(!text.contains(secret));
                }
            }
            if !keyed {
                assert!(!store.root().join("requests").exists());
            }
        }
    }
}

#![cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[path = "common/managed.rs"]
mod managed;
use qq_cli::requests::ledger::{self, Outcome};
use qq_cli::storage::credentials::{Credentials, SystemCredentials};
use qq_cli::storage::state::{Installation, Instance, Store};
use serde_json::{Value, json};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run(config: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .env_remove("QQ_CLI_CONFIG")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(8);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Synthetic send CLI did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains("synthetic-cli-token"));
    (
        output.status.code().unwrap(),
        serde_json::from_str(text).unwrap(),
    )
}

#[test]
fn send_cli_uses_temporary_vault_and_caches_private_group_attachment_outcomes_offline() {
    if managed::child() {
        return;
    }
    write_flow(false);
}

#[test]
fn reply_cli_infers_scope_across_multiple_instances_and_caches_attachment_outcomes() {
    if managed::child() {
        return;
    }
    write_flow(true);
}

fn write_flow(reply: bool) {
    for private in [false, true] {
        for uncertain in [false, true] {
            struct Cleanup(SystemCredentials);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    let _ = self.0.delete("synthetic-send");
                    let _ = self.0.delete("reference-signing-key");
                }
            }
            let temp = tempfile::tempdir().unwrap();
            let store = Store::new(temp.path().join("state"));
            let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
            credentials
                .0
                .set("synthetic-send", "synthetic-cli-token")
                .unwrap();
            let state = store.root().to_owned();
            let (endpoint, server) = http::serve(
                (if uncertain { 4 } else { 7 }) + if reply { 7 } else { 0 },
                move |action, payload| {
                    let data = match action {
                        "get_version_info" => json!({"app_version":"4.18.38"}),
                        "get_status" => json!({"online":true}),
                        "get_login_info" => json!({"user_id":"10001"}),
                        "get_msg" => {
                            assert!(reply);
                            assert_eq!(payload["message_id"], "40000");
                            let mut message = json!({"message_id":"40000","real_seq":"100","time":1700000000,"self_id":"10001","user_id":"10002","message_type":if private {"private"}else{"group"},"message":[{"type":"text","data":{"text":"synthetic reply target"}}]});
                            if private {
                                message["target_id"] = json!("10001");
                            } else {
                                message["group_id"] = json!("20001");
                            }
                            message
                        }
                        "send_private_msg" | "send_group_msg" => {
                            assert_eq!(
                                action,
                                if private {
                                    "send_private_msg"
                                } else {
                                    "send_group_msg"
                                }
                            );
                            assert_eq!(
                                payload[if private { "user_id" } else { "group_id" }],
                                if private { "10002" } else { "20001" }
                            );
                            assert_eq!(
                                ledger::get(&Store::new(state.clone()), "cli-send-key")
                                    .unwrap()
                                    .outcome,
                                Outcome::Pending
                            );
                            let segments = payload["message"].as_array().unwrap();
                            let offset = usize::from(reply);
                            assert_eq!(segments.len(), 3 + offset);
                            if reply {
                                assert_eq!(segments[0]["type"], "reply");
                                assert_eq!(segments[0]["data"]["id"], "40000");
                            }
                            assert_eq!(segments[offset]["type"], "text");
                            assert_eq!(segments[offset + 1]["type"], "image");
                            assert_eq!(segments[offset + 2]["type"], "file");
                            for segment in &segments[offset + 1..] {
                                let path =
                                    std::path::Path::new(segment["data"]["file"].as_str().unwrap());
                                assert_eq!(
                                    std::fs::read(path).unwrap(),
                                    b"synthetic attachment input"
                                );
                                assert!(path.starts_with(
                                    state.join("attachments").canonicalize().unwrap()
                                ));
                            }
                            if uncertain {
                                return b"{".to_vec();
                            }
                            json!({"message_id":"9007199254740993"})
                        }
                        _ => panic!("Unexpected synthetic action"),
                    };
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
                },
            );
            let mut transaction = store
                .transaction(Instant::now() + Duration::from_secs(3))
                .unwrap();
            transaction.registry.installations.push(Installation {
                id: "install-example".into(),
                version: "v4.18.38".into(),
                digest: "a".repeat(64),
                directory: temp.path().join("unused-installation"),
            });
            transaction.registry.instances.push(Instance {
                id: "instance-example".into(),
                installation_id: "install-example".into(),
                endpoint,
                ws_endpoint: None,
                credential_key: "synthetic-send".into(),
                account: Some("10001".into()),
                run_id: "run-example".into(),
                state: "ready".into(),
                process_id: None,
                process_started: None,
            });
            if reply {
                let mut other = transaction.registry.instances[0].clone();
                other.id = "other-instance".into();
                other.credential_key = "synthetic-key-that-does-not-exist".into();
                other.account = Some("10003".into());
                transaction.registry.instances.push(other);
            }
            transaction.commit().unwrap();
            let _managed = managed::Managed::attach(
                &store,
                &credentials.0,
                "instance-example",
                if reply {
                    "reply_cli_infers_scope_across_multiple_instances_and_caches_attachment_outcomes"
                } else {
                    "send_cli_uses_temporary_vault_and_caches_private_group_attachment_outcomes_offline"
                },
            );
            let token = if reply {
                let refs = qq_cli::storage::references::References::open(
                    &store,
                    &credentials.0,
                    Instant::now() + Duration::from_secs(3),
                )
                .unwrap();
                refs.issue(
                    "m1",
                    &qq_cli::messaging::message::Locator {
                        scope: qq_cli::core::domain::Scope {
                            instance_id: "instance-example".into(),
                            account: "10001".into(),
                        },
                        run_id: "run-example".into(),
                        conversation: if private {
                            qq_cli::core::domain::Conversation::Private("10002".into())
                        } else {
                            qq_cli::core::domain::Conversation::Group("20001".into())
                        },
                        backend_id: "40000".into(),
                        time_seconds: Some(1700000000),
                        sequence: Some("100".into()),
                        sender_id: Some("10002".into()),
                    },
                )
                .unwrap()
            } else {
                String::new()
            };
            let config = temp.path().join("config.json");
            std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
            let source = temp.path().join("fixture.bin");
            std::fs::write(&source, b"synthetic attachment input").unwrap();
            let mut args = if reply {
                vec!["message", "reply", token.as_str()]
            } else {
                vec![
                    "message",
                    "send",
                    "--conversation",
                    if private {
                        "private:10002"
                    } else {
                        "group:20001"
                    },
                ]
            };
            args.extend([
                "--text",
                "synthetic CLI text",
                "--image",
                source.to_str().unwrap(),
                "--file",
                source.to_str().unwrap(),
                "--request-id",
                "cli-send-key",
            ]);
            let (code, first) = run(&config, &args);
            let descriptor = qq_cli::cli::registry::get(if reply {
                "message.reply"
            } else {
                "message.send"
            })
            .unwrap();
            let response_schema = jsonschema::validator_for(&descriptor["responseSchema"]).unwrap();
            assert!(response_schema.is_valid(&first), "{first}");
            assert_eq!(first["meta"]["scope"]["account"], "10001");
            if uncertain {
                assert_eq!(first["error"]["nextAction"]["command"], "request.get");
                assert_eq!(
                    first["error"]["nextAction"]["args"]["request_id"],
                    "cli-send-key"
                );
                let next = qq_cli::cli::registry::get("request.get").unwrap();
                assert!(
                    jsonschema::validator_for(&next["inputSchema"])
                        .unwrap()
                        .is_valid(&first["error"]["nextAction"]["args"])
                );
            }
            assert_eq!(code, if uncertain { 5 } else { 0 }, "{first}");
            server.join().unwrap();
            // Known receipts remain usable while stopped, without contacting the API.
            let mut transaction = store
                .transaction(Instant::now() + Duration::from_secs(3))
                .unwrap();
            transaction.registry.instances[0].state = "stopped".into();
            // A missing managed runtime must not prevent an offline matching receipt.
            transaction.registry.instances[0].credential_key =
                qq_cli::napcat::runtime::runtime_store::api_credential_key(
                    &transaction.registry.instances[0].id,
                    &transaction.registry.instances[0].run_id,
                )
                .unwrap();
            transaction.commit().unwrap();
            let (second_code, second) = run(&config, &args);
            assert!(response_schema.is_valid(&second), "{second}");
            assert_eq!(second_code, code);
            assert_eq!(second["meta"]["scope"], first["meta"]["scope"]);
            if reply {
                let mut conflict = args.clone();
                conflict.extend(["--instance", "other-instance"]);
                let (code, error) = run(&config, &conflict);
                assert_eq!(code, 2);
                assert_eq!(error["error"]["code"], "INSTANCE_CONFLICT");
            }
            if uncertain {
                assert_eq!(second["error"]["code"], "WRITE_OUTCOME_UNKNOWN");
            } else {
                assert_eq!(second["data"]["cached"], true);
            }
            let (code, record) = run(&config, &["request", "get", "cli-send-key"]);
            let descriptor = qq_cli::cli::registry::get("request.get").unwrap();
            assert!(
                jsonschema::validator_for(&descriptor["responseSchema"])
                    .unwrap()
                    .is_valid(&record),
                "{record}"
            );
            assert_eq!(code, 0);
            assert_eq!(
                record["data"]["outcome"],
                if uncertain { "unknown" } else { "accepted" }
            );
            assert_eq!(
                record["data"]["receipt"]["fileTransfer"],
                if uncertain {
                    "unknown"
                } else {
                    "not_observable"
                }
            );
            assert_eq!(record["meta"]["scope"]["account"], "10001");
            assert!(!first.to_string().contains("prepare-"));
            assert_eq!(
                std::fs::read_dir(store.root().join("attachments"))
                    .unwrap()
                    .count(),
                2
            );
            credentials.0.delete("synthetic-send").unwrap();
            credentials.0.delete("reference-signing-key").unwrap();
        }
    }
}

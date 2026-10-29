use serde_json::{Value, json};
use std::process::{Command, Stdio};
fn run(args: &[&str], config: &std::path::Path) -> (i32, Vec<Value>) {
    let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect(),
    )
}
#[test]
fn raw_stream_previews_and_stage_previews_are_offline_with_real_schemas() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    assert_eq!(run(&["api", "stream", "unknown_stream"], &config).0, 2);
    let (code, rows) = run(&["api", "stream", "unknown_stream", "--dry-run"], &config);
    assert_eq!(code, 0);
    assert_eq!(rows.len(), 1);
    let schema = qq_cli::cli::registry::get("api.stream").unwrap();
    assert_eq!(schema["outputFormat"], "ndjson");
    assert!(
        jsonschema::validator_for(&schema["responseSchema"])
            .unwrap()
            .is_valid(&rows[0])
    );
    let file = t.path().join("stage.bin");
    std::fs::write(&file, b"synthetic-stage").unwrap();
    let (code, rows) = run(
        &["file", "stage", file.to_str().unwrap(), "--dry-run"],
        &config,
    );
    assert_eq!(code, 0, "{rows:?}");
    assert_eq!(rows[0]["data"]["operation"], "file.stage");
    assert_eq!(std::fs::read_dir(t.path()).unwrap().count(), 1);
    for action in [
        "upload_file_stream",
        "download_file_stream",
        "download_file_image_stream",
        "download_file_record_stream",
        "test_download_stream",
    ] {
        let (_, rows) = run(&["api", "describe", action], &config);
        assert_eq!(rows[0]["data"]["responseMode"], "stream");
        assert_eq!(rows[0]["data"]["parametersKnown"], true);
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
    use base64::{Engine, engine::general_purpose::STANDARD};
    use qq_cli::{
        core::domain::Scope,
        storage::{
            credentials::{Credentials, SystemCredentials},
            references::References,
            state::{Installation, Instance, Store},
        },
    };
    use sha2::{Digest, Sha256};
    use std::time::{Duration, Instant};
    struct Vault(SystemCredentials);
    impl Drop for Vault {
        fn drop(&mut self) {
            let _ = self.0.delete("synthetic-streams");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    fn message(record: bool) -> Value {
        json!({"message_id":"40000","real_seq":"100","time":1700000000,"self_id":"10001","user_id":"10002","message_type":"private","target_id":"10001","message":[{"type":if record{"record"}else{"file"},"data":{"file":"opaque-media-alias","file_id":"opaque-media-alias","file_size":"3"}}]})
    }
    fn ok(data: Value) -> Value {
        json!({"status":"ok","retcode":0,"data":data,"stream":"stream-action"})
    }
    fn wire(rows: Vec<Value>) -> Vec<u8> {
        let mut bytes = Vec::new();
        for row in rows {
            bytes.extend(serde_json::to_vec(&row).unwrap());
            bytes.extend(b"\r\n\r\n");
        }
        bytes
    }
    #[test]
    fn managed_native_streams_preserve_raw_packets_and_publish_only_verified_transfers() {
        if managed::child() {
            return;
        }
        for mode in [
            "raw-ok",
            "raw-cut",
            "raw-failed",
            "raw-async",
            "raw-truncated",
            "raw-after-final",
            "raw-limit",
            "raw-changed",
            "raw-upload",
            "raw-upload-id",
            "raw-no-media",
            "raw-receive-no-read",
            "stage-ok",
            "stage-cut",
            "stage-hash",
            "stage-changed",
            "stage-denied",
            "stage-preflight",
            "media-file",
            "media-record",
            "media-index",
            "media-cut",
            "media-changed",
            "media-denied",
        ] {
            let t = tempfile::tempdir().unwrap();
            let root = t.path().join("state");
            let native_root = root.clone();
            let raw = mode.starts_with("raw-");
            let stage = mode.starts_with("stage-");
            let media = mode.starts_with("media-");
            let denied = matches!(
                mode,
                "stage-denied" | "media-denied" | "raw-no-media" | "raw-receive-no-read"
            );
            let expected = if denied {
                0
            } else if raw {
                if matches!(mode, "raw-truncated" | "raw-after-final" | "raw-limit") {
                    4
                } else {
                    7
                }
            } else if stage {
                if mode == "stage-preflight" {
                    3
                } else if mode == "stage-cut" {
                    5
                } else if mode == "stage-hash" {
                    7
                } else {
                    10
                }
            } else if mode == "media-index" {
                11
            } else if matches!(mode, "media-cut" | "media-changed") {
                14
            } else {
                17
            };
            let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let observed = calls.clone();
            let mut stage_id = String::new();
            let mut filename = String::new();
            let mut received = Vec::new();
            let mut total = 0_u64;
            let source = vec![7_u8; 70000];
            let digest = format!("{:x}", Sha256::digest(&source));
            let native_digest = digest.clone();
            let (endpoint, server) = http::serve(expected, move |action, payload| {
                if action == "get_status" {
                    return serde_json::to_vec(&ok(json!({"online":true}))).unwrap();
                }
                if action == "get_login_info" {
                    let changed = mode == "stage-preflight"
                        || mode == "raw-changed"
                            && observed.load(std::sync::atomic::Ordering::SeqCst) > 0
                        || mode == "stage-changed"
                            && observed.load(std::sync::atomic::Ordering::SeqCst) >= 4
                        || mode == "media-changed"
                            && observed.load(std::sync::atomic::Ordering::SeqCst) >= 2;
                    return serde_json::to_vec(&ok(
                        json!({"user_id":if changed{"10003"}else{"10001"}}),
                    ))
                    .unwrap();
                }
                if action == "get_version_info" {
                    return serde_json::to_vec(&ok(json!({"app_version":"4.18.38"}))).unwrap();
                }
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if action == "get_msg" {
                    return serde_json::to_vec(&ok(message(mode == "media-record"))).unwrap();
                }
                if raw {
                    if matches!(mode, "raw-upload" | "raw-upload-id") {
                        assert_eq!(action, "upload_file_stream");
                        return wire(vec![ok(
                            json!({"type":"stream","stream_id":if mode=="raw-upload-id"{"different-stream"}else{"synthetic-stream"},"status":"chunk_received","received_chunks":1,"total_chunks":2}),
                        )]);
                    }
                    assert_eq!(action, "future_stream");
                    assert_eq!(payload["syntheticInput"], "synthetic-private-param");
                    let chunk = ok(
                        json!({"type":"stream","data_type":"data_chunk","secret":"synthetic-native-secret","value":"中文"}),
                    );
                    let complete = ok(json!({"type":"response","data_type":"data_complete"}));
                    match mode {
                        "raw-cut" => wire(vec![chunk]),
                        "raw-failed" => wire(vec![
                            chunk,
                            json!({"status":"failed","retcode":1200,"data":{"type":"error"},"message":"synthetic-native-secret"}),
                        ]),
                        "raw-async" => {
                            wire(vec![json!({"status":"async","retcode":1,"data":null})])
                        }
                        "raw-truncated" => {
                            let mut b = wire(vec![chunk]);
                            b.extend(b"{\"status\":");
                            b
                        }
                        "raw-after-final" => wire(vec![complete, chunk]),
                        "raw-limit" => {
                            wire(vec![ok(json!({"type":"stream","text":"x".repeat(2048)}))])
                        }
                        _ => wire(vec![chunk, complete]),
                    }
                } else if stage {
                    assert_eq!(action, "upload_file_stream");
                    assert_eq!(payload["file_retention"], 300000.0);
                    let status = if payload.get("filename").is_some() {
                        stage_id = payload["stream_id"].as_str().unwrap().to_owned();
                        filename = payload["filename"].as_str().unwrap().to_owned();
                        assert!(stage_id.starts_with("qq-cli-"));
                        assert!(!filename.contains(['/', '\\', ':']));
                        assert_eq!(payload["expected_sha256"], native_digest);
                        total = payload["total_chunks"].as_f64().unwrap() as u64;
                        assert_eq!(total, 2);
                        let record = qq_cli::requests::ledger::get(
                            &Store::new(native_root.clone()),
                            "stream-example",
                        )
                        .unwrap();
                        assert_eq!(
                            record.outcome,
                            qq_cli::core::contracts::write::Outcome::Pending
                        );
                        assert_eq!(
                            std::fs::read_dir(native_root.join("attachments"))
                                .unwrap()
                                .count(),
                            1
                        );
                        "file_created"
                    } else if payload.get("chunk_data").is_some() {
                        assert_eq!(payload["stream_id"], stage_id);
                        let index = payload["chunk_index"].as_f64().unwrap() as usize;
                        assert_eq!(index, received.len().div_ceil(65536));
                        if mode == "stage-cut" {
                            return wire(vec![
                                json!({"status":"failed","retcode":1200,"data":{"type":"error"}}),
                            ]);
                        }
                        received.extend(
                            STANDARD
                                .decode(payload["chunk_data"].as_str().unwrap())
                                .unwrap(),
                        );
                        "chunk_received"
                    } else {
                        assert_eq!(payload["is_complete"], true);
                        assert_eq!(received, vec![7_u8; 70000]);
                        "file_complete"
                    };
                    wire(vec![ok(
                        json!({"type":if status=="file_complete"{"response"}else{"stream"},"stream_id":stage_id,"status":status,"received_chunks":if status=="file_created"{0}else if status=="file_complete"{total}else{received.len().div_ceil(65536) as u64},"total_chunks":total,"file_path":format!("/synthetic-backend/{filename}"),"file_size":received.len(),"sha256":if mode=="stage-hash"{"incorrect-hash"}else{native_digest.as_str()}}),
                    )])
                } else {
                    assert!(media);
                    assert_eq!(
                        action,
                        if mode == "media-record" {
                            "download_file_record_stream"
                        } else {
                            "download_file_stream"
                        }
                    );
                    assert_eq!(payload["file_id"], "opaque-media-alias");
                    assert_eq!(payload["chunk_size"], 65536.0);
                    if mode == "media-record" {
                        assert_eq!(payload["out_format"], "mp3");
                    }
                    let bytes = if mode == "media-record" {
                        b"hello".as_slice()
                    } else {
                        b"abc".as_slice()
                    };
                    let text = STANDARD.encode(bytes);
                    let mut rows = vec![
                        ok(
                            json!({"type":"stream","data_type":"file_info","file_name":"../../ignored-backend-name","file_size":3,"chunk_size":65536,"out_format":if mode=="media-record"{"mp3"}else{""}}),
                        ),
                        ok(
                            json!({"type":"stream","data_type":"file_chunk","index":if mode=="media-index"{1}else{0},"data":text,"size":bytes.len(),"base64_size":text.len()}),
                        ),
                    ];
                    if mode != "media-cut" {
                        rows.push(ok(json!({"type":"response","data_type":"file_complete","total_chunks":1,"total_bytes":bytes.len()})));
                    }
                    wire(rows)
                }
            });
            let store = Store::new(root);
            let vault = Vault(SystemCredentials::new(store.root()).unwrap());
            vault
                .0
                .set("synthetic-streams", "synthetic-cli-token")
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
                credential_key: "synthetic-streams".into(),
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
                "owned::managed_native_streams_preserve_raw_packets_and_publish_only_verified_transfers",
            );
            let refs = References::open(&store, &vault.0, Instant::now() + Duration::from_secs(5))
                .unwrap();
            let media_token = if media {
                let scope = Scope {
                    account: "10001".into(),
                    instance_id: "instance-example".into(),
                };
                let message = qq_cli::messaging::message::normalize(
                    &message(mode == "media-record"),
                    &scope,
                    "run-example",
                    &"private:10002".parse().unwrap(),
                    &refs,
                )
                .unwrap();
                serde_json::to_value(message.value).unwrap()["segments"][0]["mediaRef"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            } else {
                String::new()
            };
            let config = t.path().join("config.json");
            std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"responseBytes":if mode=="raw-limit"{1024}else{1048576},"permissions":{"send":mode!="stage-denied","read":!raw,"mediaDownload":mode!="media-denied"&&mode!="raw-no-media","targets":if media{vec!["private:10002"]}else{vec![]}}})).unwrap()).unwrap();
            let params = t.path().join("params.json");
            std::fs::write(&params,serde_json::to_vec(&if matches!(mode,"raw-upload"|"raw-upload-id"){json!({"stream_id":"synthetic-stream","chunk_data":"YQ==","chunk_index":0,"total_chunks":2,"file_retention":300000})}else{json!({"syntheticInput":"synthetic-private-param"})}).unwrap()).unwrap();
            let source = t.path().join("stage.bin");
            std::fs::write(&source, vec![7_u8; 70000]).unwrap();
            let output = t.path().join("output.bin");
            let args = if raw {
                vec![
                    "api",
                    "stream",
                    if mode == "raw-receive-no-read" {
                        "receive_online_file"
                    } else if mode == "raw-no-media" {
                        "get_record"
                    } else if matches!(mode, "raw-upload" | "raw-upload-id") {
                        "upload_file_stream"
                    } else {
                        "future_stream"
                    },
                    "--params-file",
                    params.to_str().unwrap(),
                    "--request-id",
                    "stream-example",
                ]
            } else if stage {
                vec![
                    "file",
                    "stage",
                    source.to_str().unwrap(),
                    "--request-id",
                    "stream-example",
                ]
            } else {
                let mut args = vec![
                    "media",
                    "download",
                    &media_token,
                    "--output",
                    output.to_str().unwrap(),
                    "--stream",
                ];
                if mode == "media-record" {
                    args.extend(["--format", "mp3"]);
                }
                args
            };
            let server = server.start();
            let (code, rows) = run(&args, &config);
            let wanted = if denied || mode == "stage-preflight" {
                4
            } else if matches!(
                mode,
                "raw-ok" | "raw-upload" | "stage-ok" | "media-file" | "media-record"
            ) {
                0
            } else if mode == "raw-failed" || mode == "media-cut" || mode == "media-changed" {
                4
            } else if mode == "media-index" {
                3
            } else {
                5
            };
            assert_eq!(code, wanted, "{mode}: {rows:?}");
            server.join().unwrap();
            let path = if raw {
                "api.stream"
            } else if stage {
                "file.stage"
            } else {
                "media.download"
            };
            let schema = qq_cli::cli::registry::get(path).unwrap();
            let validator = jsonschema::validator_for(&schema["responseSchema"]).unwrap();
            for row in &rows {
                assert!(validator.is_valid(row), "{mode}: {row}");
            }
            if raw && !denied {
                assert_eq!(rows.first().unwrap()["data"]["type"], "ready");
                assert_eq!(rows.last().unwrap()["data"]["type"], "summary");
                if mode == "raw-ok" {
                    assert!(
                        rows.iter()
                            .any(|r| r.to_string().contains("synthetic-native-secret"))
                    );
                }
                if mode == "raw-upload" {
                    assert_eq!(rows.last().unwrap()["data"]["transferComplete"], false);
                }
            }
            if media {
                if code == 0 {
                    assert_eq!(
                        std::fs::read(&output).unwrap(),
                        if mode == "media-record" {
                            b"hello".as_slice()
                        } else {
                            b"abc".as_slice()
                        }
                    );
                } else {
                    assert!(!output.exists());
                }
            } else if !denied {
                if mode == "raw-ok" {
                    let (other, rows) = run(
                        &[
                            "api",
                            "call",
                            "future_stream",
                            "--params-file",
                            params.to_str().unwrap(),
                            "--request-id",
                            "stream-example",
                        ],
                        &config,
                    );
                    assert_eq!(other, 0, "{rows:?}");
                    assert_eq!(rows[0]["data"]["deduplicated"], true);
                    assert_eq!(rows[0]["data"]["resultAvailable"], false);
                }
                let (cached, cached_rows) = run(&args, &config);
                assert_eq!(cached, code, "{mode}: {cached_rows:?}");
                assert_eq!(cached_rows.len(), 1);
                if raw {
                    assert_eq!(cached_rows[0]["data"]["packetsAvailable"], false);
                } else if code == 0 {
                    assert_eq!(rows[0]["data"]["sha256"], digest);
                    assert_eq!(cached_rows[0]["data"]["pathAvailable"], false);
                }
                let record = qq_cli::requests::ledger::get(&store, "stream-example").unwrap();
                let text = serde_json::to_string(&record).unwrap();
                for secret in [
                    "synthetic-native-secret",
                    "synthetic-private-param",
                    "synthetic-backend",
                    "stage.bin",
                    "chunk_data",
                ] {
                    assert!(!text.contains(secret), "{text}");
                }
            }
        }
    }
}

#![cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use qq_cli::storage::credentials::{Credentials, SystemCredentials};
use qq_cli::storage::state::{Installation, Instance, Store};
use serde_json::{Value, json};

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return stream;
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("Synthetic event server failed: {error}"),
        }
    }
}

#[test]
fn event_watch_cli_streams_fake_session_with_isolated_native_credentials() {
    if managed::child() {
        return;
    }
    for mode in ["bounded", "blocked", "continuous"] {
        let blocked = mode == "blocked";
        let continuous = mode == "continuous";
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-events");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
        credentials
            .0
            .set("synthetic-events", "synthetic-event-token")
            .unwrap();
        let http = TcpListener::bind("127.0.0.1:0").unwrap();
        http.set_nonblocking(true).unwrap();
        let ws = TcpListener::bind("127.0.0.1:0").unwrap();
        ws.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/", http.local_addr().unwrap());
        let ws_endpoint = format!("ws://{}/event", ws.local_addr().unwrap());
        let mut transaction = store
            .transaction(Instant::now() + Duration::from_secs(3))
            .unwrap();
        transaction.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("unused-synthetic-installation"),
        });
        transaction.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: Some(ws_endpoint),
            credential_key: "synthetic-events".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        transaction.commit().unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &credentials.0,
            "instance-example",
            "event_watch_cli_streams_fake_session_with_isolated_native_credentials",
        );
        let http_server = std::thread::spawn(move || {
            for _ in 0..6 {
                let mut stream = accept(&http);
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                    assert!(request.len() < 65536);
                }
                let request = String::from_utf8(request).unwrap();
                let length = request
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut payload = vec![0; length];
                stream.read_exact(&mut payload).unwrap();
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer synthetic-event-token")
                );
                let data = if request.starts_with("POST /get_version_info ") {
                    json!({"app_version":"4.18.38"})
                } else if request.starts_with("POST /get_status ") {
                    json!({"online":true})
                } else {
                    assert!(request.starts_with("POST /get_login_info "));
                    json!({"user_id":"10001"})
                };
                let body =
                    serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let ws_server = std::thread::spawn(move || {
            let mut socket = tungstenite::accept(accept(&ws)).unwrap();
            let text = if blocked {
                "x".repeat(1024 * 1024)
            } else {
                "synthetic event CLI".into()
            };
            let event = json!({"post_type":"message","self_id":"10001","user_id":"10002","message_id":"40001","real_seq":"101","time":1700000000,"message_type":"group","group_id":"20001","message":[{"type":"text","data":{"text":text}}]});
            socket
                .send(tungstenite::Message::Text(event.to_string().into()))
                .unwrap();
            if continuous {
                let mut next = event.clone();
                next["message_id"] = json!("40002");
                next["real_seq"] = json!("102");
                socket
                    .send(tungstenite::Message::Text(next.to_string().into()))
                    .unwrap();
                socket
                    .send(tungstenite::Message::Binary(vec![0].into()))
                    .unwrap();
            }
            let _ = socket.read();
        });
        let config = temp.path().join("config.json");
        std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(if continuous {
                vec!["event", "watch", "--continuous", "--config"]
            } else {
                vec![
                    "event",
                    "watch",
                    "--limit",
                    "1",
                    "--timeout",
                    "3",
                    "--config",
                ]
            })
            .arg(&config)
            .env_remove("QQ_CLI_CONFIG")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(6);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= until {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("Synthetic event CLI did not exit");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        if blocked {
            assert_eq!(output.status.code(), Some(6));
            assert!(output.stderr.is_empty());
            http_server.join().unwrap();
            ws_server.join().unwrap();
            credentials.0.delete("synthetic-events").unwrap();
            credentials.0.delete("reference-signing-key").unwrap();
            continue;
        }
        assert!(
            if continuous {
                output.status.code() == Some(3)
            } else {
                output.status.success()
            },
            "Synthetic CLI exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(output.stderr.is_empty());
        let records = std::str::from_utf8(&output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        let schema = qq_cli::cli::schema::event_stream();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for record in &records {
            assert!(
                validator.is_valid(record),
                "CLI record violates schema: {record}"
            );
        }
        assert_eq!(
            records
                .iter()
                .map(|line| line["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            if continuous {
                vec!["ready", "message", "message", "summary"]
            } else {
                vec!["ready", "message", "summary"]
            }
        );
        assert_eq!(records[0]["scope"]["account"], "10001");
        assert_eq!(
            records.last().unwrap()["reason"],
            if continuous {
                "transport_error"
            } else {
                "limit"
            }
        );
        assert!(
            !String::from_utf8(output.stdout)
                .unwrap()
                .contains("synthetic-event-token")
        );
        http_server.join().unwrap();
        ws_server.join().unwrap();
        credentials.0.delete("synthetic-events").unwrap();
        credentials.0.delete("reference-signing-key").unwrap();
    }
}

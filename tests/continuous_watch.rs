use clap::Parser;
use qq_cli::cli::args::Cli;
use serde_json::json;

#[test]
fn continuous_is_explicit_and_conflicts_with_finite_budgets_in_parser_and_schema() {
    let cli = Cli::try_parse_from(["qq-cli", "event", "watch", "--continuous"]).unwrap();
    cli.validate().unwrap();
    let contract = qq_cli::cli::registry::get("event.watch").unwrap();
    let validator = jsonschema::validator_for(&contract["inputSchema"]).unwrap();
    assert!(validator.is_valid(&json!({"continuous":true})));
    for field in ["timeout", "limit"] {
        assert!(
            Cli::try_parse_from([
                "qq-cli",
                "event",
                "watch",
                "--continuous",
                &format!("--{field}"),
                "1"
            ])
            .is_err()
        );
        let mut invalid = json!({"continuous":true});
        invalid[field] = json!(1);
        assert!(!validator.is_valid(&invalid));
    }
    Cli::try_parse_from(["qq-cli", "event", "watch", "--timeout", "2", "--limit", "3"])
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn continuous_socket_crosses_total_frame_and_wire_budgets_while_bounded_mode_keeps_them() {
    use qq_cli::transport::websocket::{Connection, Frame};
    use std::time::{Duration, Instant};
    for (continuous, count, size) in [
        (false, 10001, 2),
        (true, 10001, 2),
        (true, 23, 3 * 1024 * 1024),
    ] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("ws://{}/event", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut socket = tungstenite::accept(listener.accept().unwrap().0).unwrap();
            for _ in 0..count {
                if socket
                    .send(tungstenite::Message::Text("x".repeat(size).into()))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut connection = Connection::connect(
            &endpoint,
            "synthetic-event-token",
            Instant::now() + Duration::from_secs(30),
        )
        .unwrap();
        for n in 0..count {
            if continuous {
                connection.renew(Instant::now() + Duration::from_secs(30));
            }
            if !continuous && n == 10000 {
                assert_eq!(
                    connection.receive().err().unwrap().code,
                    "EVENT_STREAM_LIMIT"
                );
            } else {
                loop {
                    match connection.receive().unwrap() {
                        Frame::Text(bytes) => {
                            assert_eq!(bytes.len(), size);
                            break;
                        }
                        Frame::Idle | Frame::Control => (),
                        Frame::Closed => panic!("synthetic event server closed early"),
                    }
                }
            }
        }
        drop(connection);
        server.join().unwrap();
    }
}

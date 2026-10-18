use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use qq_cli::websocket::{Connection, Frame, validate_endpoint};
use tungstenite::Message;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn oversized_upgrade_headers_are_rejected_before_the_stream_budget_is_enabled() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("ws://{}/event", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() < 8192);
        }
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 101 Switching Protocols\r\nX-Original: {}\r\n\r\n",
                "x".repeat(40 * 1024)
            )
            .as_bytes(),
        );
    });
    assert_eq!(
        Connection::connect(&endpoint, "original-token", deadline())
            .err()
            .unwrap()
            .code,
        "WS_TRANSPORT"
    );
    server.join().unwrap();
}

// The dependency's Callback trait fixes this unboxed HTTP error response type.
#[allow(clippy::result_large_err)]
fn inspect_handshake(
    request: &tungstenite::handshake::server::Request,
    response: tungstenite::handshake::server::Response,
) -> std::result::Result<
    tungstenite::handshake::server::Response,
    tungstenite::handshake::server::ErrorResponse,
> {
    assert_eq!(request.uri().path(), "/event");
    assert!(request.uri().query().is_none());
    assert_eq!(request.headers()["authorization"], "Bearer synthetic-token");
    Ok(response)
}
fn listener() -> (TcpListener, String) {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    socket.set_nonblocking(true).unwrap();
    let url = format!("ws://{}/event", socket.local_addr().unwrap());
    (socket, url)
}
fn accept(listener: TcpListener) -> TcpStream {
    let until = deadline();
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
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until =>
            {
                thread::sleep(Duration::from_millis(5))
            }
            Err(error) => panic!("Synthetic listener failed: {error}"),
        }
    }
}

#[test]
fn websocket_endpoint_rejects_remote_implicit_and_credential_bearing_urls() {
    for invalid in [
        "ws://localhost:16000/",
        "ws://127.0.0.1/",
        "wss://127.0.0.1:16000/",
        "ws://127.0.0.1:16000/?token=synthetic",
        "ws://user:secret@127.0.0.1:16000/",
        "ws://127.0.0.1:16000/api",
        "ws://127.0.0.1:16000/event#fragment",
        "ws://127.0.0.1:0/",
    ] {
        assert!(validate_endpoint(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        validate_endpoint("ws://[::1]:16000/event/").unwrap().0,
        "ws://[::1]:16000/event"
    );
    assert_eq!(
        Connection::connect("ws://127.0.0.1:16000/", "bad\r\nheader", deadline())
            .err()
            .unwrap()
            .code,
        "CREDENTIAL_INVALID"
    );
}

#[test]
fn websocket_handshake_uses_header_auth_and_handles_ping_text_and_close() {
    let (listener, url) = listener();
    let server = thread::spawn(move || {
        let stream = accept(listener);
        let mut socket = tungstenite::accept_hdr(stream, inspect_handshake).unwrap();
        socket.send(Message::Ping(vec![1, 2, 3].into())).unwrap();
        assert!(matches!(socket.read().unwrap(), Message::Pong(_)));
        socket
            .send(Message::Text("synthetic event frame".into()))
            .unwrap();
        socket.close(None).unwrap();
        let _ = socket.read();
    });
    let mut connection = Connection::connect(&url, "synthetic-token", deadline()).unwrap();
    assert!(matches!(connection.receive().unwrap(), Frame::Control));
    match connection.receive().unwrap() {
        Frame::Text(text) => assert_eq!(text, b"synthetic event frame"),
        _ => panic!("missing text"),
    }
    assert!(matches!(connection.receive().unwrap(), Frame::Closed));
    drop(connection);
    server.join().unwrap();
}

#[test]
fn websocket_refuses_auth_and_redirect_without_replaying_handshake() {
    for (status, code) in [
        ("401 Unauthorized", "BACKEND_AUTH"),
        ("302 Found", "WS_HANDSHAKE"),
    ] {
        let (listener, url) = listener();
        let server = thread::spawn(move || {
            let mut stream = accept(listener);
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() < 65536);
            }
            stream.write_all(format!("HTTP/1.1 {status}\r\nLocation: http://invalid.example/\r\nContent-Length: 0\r\n\r\n").as_bytes()).unwrap();
        });
        assert_eq!(
            Connection::connect(&url, "synthetic-token", deadline())
                .err()
                .unwrap()
                .code,
            code
        );
        server.join().unwrap();
    }
}

#[test]
fn websocket_rejects_binary_and_oversized_messages_and_obeys_total_deadline() {
    for mode in ["binary", "oversized", "idle"] {
        let (listener, url) = listener();
        let server = thread::spawn(move || {
            let mut socket = tungstenite::accept(accept(listener)).unwrap();
            match mode {
                "binary" => {
                    let _ = socket.send(Message::Binary(vec![1, 2].into()));
                }
                "oversized" => {
                    let _ = socket.send(Message::Text("x".repeat(4 * 1024 * 1024 + 1).into()));
                }
                _ => {
                    let _ = socket.read();
                }
            }
        });
        let until = if mode == "idle" {
            Instant::now() + Duration::from_millis(300)
        } else {
            deadline()
        };
        let mut connection = Connection::connect(&url, "synthetic-token", until).unwrap();
        let error = connection.receive().err().unwrap();
        assert_eq!(
            error.code,
            match mode {
                "binary" => "WS_PROTOCOL",
                "oversized" => "EVENT_STREAM_LIMIT",
                _ => "DEADLINE",
            }
        );
        drop(connection);
        server.join().unwrap();
    }
}

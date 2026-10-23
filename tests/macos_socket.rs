#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::native::{macos_socket::bind_connected, process::Pinned};
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, sync_channel};
use std::time::{Duration, Instant};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(4)
}
struct Server {
    child: Child,
    lines: Receiver<String>,
    address: SocketAddr,
}
impl Server {
    fn spawn(mode: &str, host: &str) -> Self {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "synthetic_socket_service",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("QQ_CLI_SOCKET_SERVICE", mode)
            .env("QQ_CLI_SOCKET_HOST", host)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = sync_channel(16);
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                if send.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        // Cleanup owns the child even if readiness or address parsing fails.
        let mut server = Self {
            child,
            lines,
            address: "127.0.0.1:1".parse().unwrap(),
        };
        server.address = server.line("endpoint:").parse().unwrap();
        server
    }
    fn line(&self, prefix: &str) -> String {
        let end = deadline();
        loop {
            let line = self
                .lines
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .unwrap();
            if let Some((_, value)) = line.split_once(prefix) {
                return value.trim().to_owned();
            }
        }
    }
    fn owner(&self) -> Pinned {
        Pinned::observe(self.child.id(), deadline()).unwrap()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
// Tungstenite's callback fixes the response error type; tests cannot box that API error.
#[allow(clippy::result_large_err)]
fn inspect_handshake(
    request: &tungstenite::handshake::server::Request,
    response: tungstenite::handshake::server::Response,
) -> Result<tungstenite::handshake::server::Response, tungstenite::handshake::server::ErrorResponse>
{
    assert_eq!(request.headers()["authorization"], "Bearer synthetic-token");
    Ok(response)
}
#[test]
fn synthetic_socket_service() {
    let Ok(mode) = std::env::var("QQ_CLI_SOCKET_SERVICE") else {
        return;
    };
    let listener = TcpListener::bind(std::env::var("QQ_CLI_SOCKET_HOST").unwrap()).unwrap();
    println!("endpoint:{}", listener.local_addr().unwrap());
    std::io::stdout().flush().unwrap();
    listener.set_nonblocking(true).unwrap();
    if mode == "listen-only" {
        std::thread::sleep(Duration::from_millis(400));
    }
    if mode == "delay" {
        std::thread::sleep(Duration::from_millis(100));
    }
    let until = deadline();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(5))
            }
            Err(e) => panic!("synthetic accept: {e}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    if mode == "ws" {
        let mut socket = tungstenite::accept_hdr(stream, inspect_handshake).unwrap();
        socket
            .send(tungstenite::Message::Text(
                "{\"post_type\":\"meta_event\"}".into(),
            ))
            .unwrap();
    } else if mode == "http" || mode == "control" {
        let mut bytes = Vec::new();
        while !bytes.ends_with(b"\r\n\r\n") {
            let mut b = [0];
            stream.read_exact(&mut b).unwrap();
            bytes.push(b[0]);
            assert!(bytes.len() < 8192);
        }
        let headers = std::str::from_utf8(&bytes).unwrap().to_ascii_lowercase();
        let count: usize = headers
            .lines()
            .find_map(|l| l.strip_prefix("content-length:"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert!(count < 4096);
        let mut body = vec![0; count];
        stream.read_exact(&mut body).unwrap();
        let response = if mode == "http" {
            assert!(headers.contains("authorization: bearer synthetic-token"));
            assert!(headers.starts_with("post /get_status "));
            r#"{"status":"ok","retcode":0,"data":{"online":true,"good":true}}"#
        } else {
            assert!(headers.starts_with("post /api/auth/login "));
            assert!(!headers.contains("authorization:"));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap()["hash"],
                "a".repeat(64)
            );
            r#"{"code":0,"data":{"Credential":"synthetic"}}"#
        };
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.len(),
            response
        )
        .unwrap();
    } else {
        let mut byte = [0];
        let count = stream.read(&mut byte).unwrap();
        println!("bytes:{count}");
        std::io::stdout().flush().unwrap();
        if count > 0 {
            stream.write_all(b"ok").unwrap();
        }
    }
    // Keep the verified service process alive through the response-side guard.
    let _ = std::io::stdin().read(&mut [0]);
}
#[test]
fn original_ipv4_and_ipv6_streams_bind_after_accept_without_reconnecting() {
    for host in ["127.0.0.1:0", "[::1]:0"] {
        let server = Server::spawn("delay", host);
        let owner = server.owner();
        let stream = TcpStream::connect_timeout(&server.address, Duration::from_secs(2)).unwrap();
        let fd = stream.as_raw_fd();
        let mut stream = bind_connected(stream, &owner, deadline()).unwrap();
        assert_eq!(stream.as_raw_fd(), fd);
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream.write_all(b"x").unwrap();
        let mut reply = [0; 2];
        stream.read_exact(&mut reply).unwrap();
        assert_eq!(&reply, b"ok");
        assert_eq!(server.line("bytes:"), "1");
    }
}
#[test]
fn wrong_process_and_expired_proofs_close_without_sending_payload() {
    for expired in [false, true] {
        let server = Server::spawn("raw", "127.0.0.1:0");
        let wrong = Pinned::observe(std::process::id(), deadline()).unwrap();
        let stream = TcpStream::connect(server.address).unwrap();
        let end = if expired {
            Instant::now()
        } else {
            Instant::now() + Duration::from_millis(100)
        };
        let error = bind_connected(stream, &wrong, end).unwrap_err();
        assert_eq!(error.code, "DEADLINE");
        assert_eq!(server.line("bytes:"), "0");
    }
}
#[test]
fn onebot_and_control_use_original_guarded_http_connection() {
    use qq_cli::{
        storage::config::Config,
        transport::{
            client::{Action, Backend},
            owned_http::OwnedOneBot,
        },
    };
    let server = Server::spawn("http", "127.0.0.1:0");
    let owner = server.owner();
    let client = OwnedOneBot::for_process(
        &owner,
        &format!("http://{}/", server.address),
        "synthetic-token",
        &Config::default(),
    )
    .unwrap();
    assert_eq!(
        client
            .call(Action::Status, &serde_json::json!({}), deadline())
            .unwrap()["online"],
        true
    );
    use qq_cli::napcat::control::{Backend as ControlBackend, Route, owned::OwnedControl};
    let server = Server::spawn("control", "127.0.0.1:0");
    let owner = server.owner();
    let client = OwnedControl::for_process(
        &owner,
        &format!("http://{}/", server.address),
        &Config::default(),
    )
    .unwrap();
    let bytes = client
        .post(
            Route::Authenticate,
            &serde_json::json!({"hash":"a".repeat(64)}),
            None,
            deadline(),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["code"],
        0
    );
}
#[test]
fn websocket_upgrade_is_bound_to_the_recorded_service() {
    let server = Server::spawn("ws", "127.0.0.1:0");
    let owner = server.owner();
    qq_cli::transport::websocket::Connection::connect_process(
        &format!("ws://{}/event", server.address),
        "synthetic-token",
        &owner,
        deadline(),
    )
    .unwrap();
}
#[test]
fn wrong_process_cannot_receive_http_or_websocket_credentials() {
    use qq_cli::{
        storage::config::Config,
        transport::{
            client::{Action, Backend},
            owned_http::OwnedOneBot,
        },
    };
    for websocket in [false, true] {
        let server = Server::spawn("raw", "127.0.0.1:0");
        let wrong = Pinned::observe(std::process::id(), deadline()).unwrap();
        let end = Instant::now() + Duration::from_millis(150);
        let error = if websocket {
            qq_cli::transport::websocket::Connection::connect_process(
                &format!("ws://{}/event", server.address),
                "synthetic-token",
                &wrong,
                end,
            )
            .err()
            .unwrap()
        } else {
            let client = OwnedOneBot::for_process(
                &wrong,
                &format!("http://{}/", server.address),
                "synthetic-token",
                &Config::default(),
            )
            .unwrap();
            client
                .call(Action::Status, &serde_json::json!({}), end)
                .unwrap_err()
        };
        assert_eq!(error.code, "DEADLINE");
        assert_eq!(server.line("bytes:"), "0");
    }
}

#[test]
fn listening_process_without_an_accepted_descriptor_cannot_authorize_the_stream() {
    let server = Server::spawn("listen-only", "127.0.0.1:0");
    let owner = server.owner();
    let stream = TcpStream::connect(server.address).unwrap();
    let error =
        bind_connected(stream, &owner, Instant::now() + Duration::from_millis(80)).unwrap_err();
    assert_eq!(error.code, "DEADLINE");
    assert_eq!(server.line("bytes:"), "0");
}

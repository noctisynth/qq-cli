use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use qq_cli::storage::config::Config;
use qq_cli::transport::client::{Action, Backend, OneBotClient};
use serde_json::json;

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        request.push(byte[0]);
        assert!(request.len() < 64 * 1024);
    }
    let headers = String::from_utf8_lossy(&request);
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    request.extend(body);
    request
}

fn serve(response: Vec<u8>) -> (String, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let mut stream = accept_bounded(&listener);
        let request = read_request(&mut stream);
        let _ = stream.write_all(&response);
        let _ = stream.flush();
        request
    });
    (endpoint, handle)
}

fn accept_bounded(listener: &TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                return stream;
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("local test listener failed: {error}"),
        }
    }
}

fn response(status: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

#[test]
fn authenticated_post_preserves_integer_precision() {
    let (endpoint, server) = serve(response(
        "200 OK",
        r#"{"status":"ok","retcode":0,"data":{"message_id":18446744073709551615}}"#,
    ));
    let client = OneBotClient::new(&endpoint, "synthetic-token", &Config::default()).unwrap();
    let result = client
        .call(
            Action::Message,
            &json!({"message_id":"-123"}),
            Instant::now() + Duration::from_secs(3),
        )
        .unwrap();
    assert_eq!(result["message_id"].as_u64(), Some(u64::MAX));
    let request = String::from_utf8(server.join().unwrap()).unwrap();
    assert!(request.starts_with("POST /get_msg HTTP/1.1"));
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer synthetic-token")
    );
    assert!(request.contains(r#""message_id":"-123""#));
}

#[test]
fn dedicated_media_budget_does_not_relax_normal_json_limits_or_follow_paths() {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(vec![b'x'; 3 * 1024 * 1024 + 3]);
    let body=json!({"status":"ok","retcode":0,"data":{"base64":encoded,"file_size":(3*1024*1024+3).to_string(),"file":"must-not-be-opened","url":"https://invalid.example/private"}}).to_string();
    assert!(body.len() > 4 * 1024 * 1024);
    let (endpoint, server) = serve(response("200 OK", &body));
    let client = OneBotClient::new(&endpoint, "synthetic-token", &Config::default()).unwrap();
    assert_eq!(
        client
            .call(
                Action::Image,
                &json!({"file_id":"fixture"}),
                Instant::now() + Duration::from_secs(5)
            )
            .unwrap_err()
            .code,
        "RESPONSE_LIMIT"
    );
    server.join().unwrap();
    let (endpoint, server) = serve(response("200 OK", &body));
    let client = OneBotClient::new(&endpoint, "synthetic-token", &Config::default()).unwrap();
    let bytes = client
        .media_bytes(
            Action::Image,
            &json!({"file_id":"fixture"}),
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
    assert_eq!(bytes.encoded.len(), encoded.len());
    assert_eq!(bytes.size, Some(3 * 1024 * 1024 + 3));
    server.join().unwrap();
}

#[test]
fn media_response_requires_bytes_final_status_and_separate_metadata_bounds() {
    for (body,code) in [
        (json!({"status":"ok","retcode":0,"data":{"file":"must-not-open","url":"https://invalid.example/"}}).to_string(),"MEDIA_BYTES_UNAVAILABLE"),
        (json!({"status":"async","retcode":1,"data":{"base64":"YQ=="}}).to_string(),"BACKEND_ASYNC"),
        (json!({"status":"ok","retcode":0,"data":{"base64":"YQ==","file_size":"bad"}}).to_string(),"BACKEND_PROTOCOL"),
        (json!({"status":"ok","retcode":0,"data":{"base64":"YQ==","ignored":"x".repeat(2048)}}).to_string(),"RESPONSE_LIMIT"),
    ] {
        let (endpoint,server)=serve(response("200 OK",&body));let config=Config {response_bytes:1024,..Config::default()};let client=OneBotClient::new(&endpoint,"synthetic-token",&config).unwrap();
        assert_eq!(client.media_bytes(Action::File,&json!({"file_id":"fixture"}),Instant::now()+Duration::from_secs(3)).err().unwrap().code,code);server.join().unwrap();
    }
}

#[test]
fn http_success_cannot_hide_rejection_or_invalid_protocol() {
    for (body, expected) in [
        (
            r#"{"status":"failed","retcode":1200,"data":null,"wording":"synthetic-private-content"}"#,
            "BACKEND_REJECTED",
        ),
        (
            r#"{"status":"async","retcode":1,"data":null}"#,
            "BACKEND_ASYNC",
        ),
        (r#"{"status":"ok","data":{}}"#, "BACKEND_PROTOCOL"),
        (
            r#"{"status":"ok","retcode":0.25,"data":{}}"#,
            "BACKEND_PROTOCOL",
        ),
        (r#"{"status":"ok","retcode":0}"#, "BACKEND_PROTOCOL"),
        ("<html>synthetic-private-content</html>", "BACKEND_PROTOCOL"),
    ] {
        let (endpoint, server) = serve(response("200 OK", body));
        let client = OneBotClient::new(&endpoint, "synthetic-token", &Config::default()).unwrap();
        let error = client
            .call(
                Action::Status,
                &json!({}),
                Instant::now() + Duration::from_secs(3),
            )
            .unwrap_err();
        assert_eq!(error.code, expected);
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("synthetic-private-content"));
        assert!(!serialized.contains("synthetic-token"));
        server.join().unwrap();
    }
}

#[test]
fn redirects_auth_failures_and_oversized_bodies_are_explicit() {
    for (response, expected) in [
        (b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(), "BACKEND_HTTP"),
        (response("401 Unauthorized", "synthetic-private-content"), "BACKEND_AUTH"),
        (response("403 Forbidden", "synthetic-private-content"), "BACKEND_AUTH"),
        (response("200 OK", &"x".repeat(1025)), "RESPONSE_LIMIT"),
        (b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nshort".to_vec(), "BACKEND_TRANSPORT"),
        (b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n401\r\n".iter().copied()
            .chain(vec![b'x';1025]).chain(b"\r\n0\r\n\r\n".iter().copied()).collect(), "RESPONSE_LIMIT"),
    ] {
        let (endpoint, server) = serve(response);
        let config = Config { response_bytes: 1024, ..Config::default() };
        let client = OneBotClient::new(&endpoint, "synthetic-token", &config).unwrap();
        let error = client.call(Action::Status, &json!({}), Instant::now() + Duration::from_secs(3)).unwrap_err();
        assert_eq!(error.code, expected);
        server.join().unwrap();
    }
}

#[test]
fn a_dropped_write_response_is_not_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let mut stream = accept_bounded(&listener);
        let request = read_request(&mut stream);
        drop(stream);
        listener.set_nonblocking(true).unwrap();
        let until = Instant::now() + Duration::from_millis(150);
        while Instant::now() < until {
            assert!(listener.accept().is_err(), "transport retried a write");
            thread::sleep(Duration::from_millis(5));
        }
        request
    });
    let client = OneBotClient::new(&endpoint, "synthetic-token", &Config::default()).unwrap();
    assert!(
        client
            .call(
                Action::SendPrivate,
                &json!({"message":[]}),
                Instant::now() + Duration::from_secs(3)
            )
            .is_err()
    );
    assert!(
        String::from_utf8(server.join().unwrap())
            .unwrap()
            .starts_with("POST /send_private_msg")
    );
}

#[test]
fn proxy_environment_does_not_redirect_loopback_credentials() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "authenticated_post_preserves_integer_precision"])
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("HTTPS_PROXY", "http://127.0.0.1:1")
        .env("ALL_PROXY", "http://127.0.0.1:1")
        .env("NO_PROXY", "")
        .output()
        .unwrap();
    assert!(output.status.success(), "proxy isolation subprocess failed");
}

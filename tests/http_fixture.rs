#[path = "common/http.rs"]
mod http;

use serde_json::json;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[test]
fn delayed_setup_does_not_expire_the_server_before_requests_start() {
    let (endpoint, server) = http::serve(1, |action, payload| {
        assert_eq!(action, "synthetic");
        assert_eq!(payload, &json!({}));
        b"{\"ok\":true}".to_vec()
    });
    // Reproduce managed setup taking longer than the five-second accept budget.
    std::thread::sleep(Duration::from_secs(6));
    let server = server.start();
    let address = endpoint.trim_start_matches("http://").trim_end_matches('/');
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream.write_all(b"POST /synthetic HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer synthetic-cli-token\r\nContent-Length: 2\r\n\r\n{}").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.ends_with("{\"ok\":true}"));
    server.join().unwrap();
}

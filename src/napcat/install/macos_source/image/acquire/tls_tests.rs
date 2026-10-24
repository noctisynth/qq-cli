use super::*;
use std::{net::TcpListener, sync::Arc, time::Duration};
fn until() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn server(response: Vec<u8>) -> (Client, std::thread::JoinHandle<Vec<u8>>) {
    use chrono::Datelike;
    use rcgen::{BasicConstraints, CertificateParams, IsCa, Issuer, KeyPair};
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let day = chrono::DateTime::from_timestamp(now, 0)
        .unwrap()
        .date_naive();
    let date = |offset| {
        let d = day + chrono::Duration::days(offset);
        rcgen::date_time_ymd(d.year(), d.month() as u8, d.day() as u8)
    };
    let mut ca = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca.not_before = date(-1);
    ca.not_after = date(7);
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![rcgen::KeyUsagePurpose::KeyCertSign];
    let ca_key = KeyPair::generate().unwrap();
    let root = ca.self_signed(&ca_key).unwrap().der().clone();
    let issuer = Issuer::new(ca, ca_key);
    let mut leaf = CertificateParams::new(vec!["qqdl.gtimg.cn".into()]).unwrap();
    leaf.not_before = date(-1);
    leaf.not_after = date(1);
    leaf.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    let key = KeyPair::generate().unwrap();
    let leaf = leaf.signed_by(&key, &issuer).unwrap().der().clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = builder(until())
        .unwrap()
        .resolve("qqdl.gtimg.cn", listener.local_addr().unwrap())
        .add_root_certificate(reqwest::Certificate::from_der(root.as_ref()).unwrap())
        .build()
        .unwrap();
    let task = std::thread::spawn(move || {
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![leaf, root],
            rustls::pki_types::PrivatePkcs8KeyDer::from(key.serialize_der()).into(),
        )
        .unwrap();
        let end = until();
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < end => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(e) => panic!("synthetic TLS listener: {e}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
        let mut stream = rustls::StreamOwned::new(connection, stream);
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            if stream.read_exact(&mut byte).is_err() {
                return request;
            }
            request.push(byte[0]);
            assert!(request.len() < 16384);
        }
        let _ = stream.write_all(&response);
        stream.conn.send_close_notify();
        let _ = stream.flush();
        request
    });
    (client, task)
}
#[test]
fn tls_transfer_uses_fixed_origin_headers_and_response_policy() {
    for (response, accepted) in [
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata",
            true,
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\ndata!",
            false,
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata",
            false,
        ),
        (
            "HTTP/1.1 302 Found\r\nLocation: https://other.invalid/file\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            false,
        ),
        (
            "HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata",
            false,
        ),
    ] {
        let (client, server) = server(response.as_bytes().to_vec());
        let result = fetch(
            &client,
            Url::parse("https://qqdl.gtimg.cn/synthetic.dmg").unwrap(),
            4,
            until(),
        );
        assert_eq!(result.is_ok(), accepted);
        if let Ok(mut response) = result {
            let mut bytes = Vec::new();
            response.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"data");
        }
        let request = String::from_utf8(server.join().unwrap())
            .unwrap()
            .to_ascii_lowercase();
        assert!(request.contains("accept-encoding: identity"));
        assert!(
            !request.contains("authorization:")
                && !request.contains("cookie:")
                && !request.contains("referer:")
        );
    }
}

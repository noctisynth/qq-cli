//! Dedicated image transport: no OneBot credentials, proxies, redirects or retries.
use crate::error::{AppError, Result};
use reqwest::blocking::{Client, ClientBuilder};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Content {
    pub reader: Box<dyn Read>,
    pub size: Option<u64>,
}
pub trait Images {
    fn open(&self, url: &str, identity: &str, deadline: Instant) -> Result<Content>;
}
pub struct Native;
impl Images for Native {
    fn open(&self, url: &str, identity: &str, deadline: Instant) -> Result<Content> {
        validate(url, identity, deadline)?;
        let client = builder().build().map_err(|_| transport())?;
        fetch(&client, url, identity, deadline)
    }
}
fn builder() -> ClientBuilder {
    Client::builder()
        .no_proxy()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
}
fn validate(url: &str, identity: &str, deadline: Instant) -> Result<()> {
    crate::deadline::check(deadline)?;
    if crate::media_source::image_identity(url).as_deref() != Some(identity) {
        return Err(AppError::new(
            "MEDIA_BINDING_UNVERIFIED",
            "Image source does not match its verified resource identity.",
            4,
        ));
    }
    Ok(())
}
fn fetch(client: &Client, url: &str, identity: &str, deadline: Instant) -> Result<Content> {
    validate(url, identity, deadline)?;
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(transport)?;
    let mut response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "image/*, application/octet-stream")
        .timeout(remaining)
        .send()
        .map_err(|_| transport())?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(AppError::new(
            "MEDIA_HTTP_REJECTED",
            "Image source did not return a final successful content response.",
            4,
        ));
    }
    let headers = response.headers();
    if headers
        .get(reqwest::header::CONTENT_ENCODING)
        .is_some_and(|value| value.as_bytes() != b"identity")
    {
        return Err(AppError::new(
            "MEDIA_HTTP_INVALID",
            "Image source returned an unsupported encoded body.",
            4,
        ));
    }
    if let Some(kind) = headers.get(reqwest::header::CONTENT_TYPE) {
        let kind = kind
            .to_str()
            .map_err(|_| invalid())?
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !(kind.starts_with("image/") || kind == "application/octet-stream") {
            return Err(invalid());
        }
    }
    let size = response.content_length();
    if size.is_some_and(|size| size == 0 || size > crate::media::MAX_DOWNLOAD_BYTES) {
        return Err(invalid());
    }
    let mut first = [0];
    response.read_exact(&mut first).map_err(|_| transport())?;
    crate::deadline::check(deadline)?;
    Ok(Content {
        reader: Box::new(std::io::Cursor::new(first).chain(response)),
        size,
    })
}
fn transport() -> AppError {
    AppError::new(
        "MEDIA_TRANSPORT",
        "Image transport failed or exceeded its deadline; no output was published.",
        3,
    )
}
fn invalid() -> AppError {
    AppError::new(
        "MEDIA_HTTP_INVALID",
        "Image source returned an unsupported content response or size.",
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{SocketAddr, TcpListener};
    const URL: &str = "https://gchat.qpic.cn/download?appid=1407&fileid=synthetic-resource&rkey=synthetic-private-key";
    // Original test-only identity; its root is never installed in system trust stores.
    fn service(
        response: Vec<u8>,
        delay: Duration,
    ) -> (SocketAddr, std::thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let task = std::thread::spawn(move || {
            use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
            let certificates = CertificateDer::pem_slice_iter(include_bytes!(
                "../tests/fixtures/synthetic-server-chain.pem"
            ))
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
            let key = PrivateKeyDer::from_pem_slice(include_bytes!(
                "../tests/fixtures/synthetic-server-pkcs8.pem"
            ))
            .unwrap();
            let config = rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(certificates, key)
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            let stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("Synthetic TLS listener: {e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let connection = rustls::ServerConnection::new(std::sync::Arc::new(config)).unwrap();
            let mut stream = rustls::StreamOwned::new(connection, stream);
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if stream.read_exact(&mut byte).is_err() {
                    return request;
                }
                request.push(byte[0]);
                assert!(request.len() < 65536);
            }
            std::thread::sleep(delay);
            let _ = stream.write_all(&response);
            stream.conn.send_close_notify();
            let _ = stream.flush();
            request
        });
        (address, task)
    }
    fn client(address: SocketAddr, host: &str, trusted: bool) -> Client {
        let mut builder = builder().resolve(host, address);
        if trusted {
            builder = builder.add_root_certificate(
                reqwest::Certificate::from_pem(include_bytes!(
                    "../tests/fixtures/synthetic-root.pem"
                ))
                .unwrap(),
            );
        }
        builder.build().unwrap()
    }
    fn until() -> Instant {
        Instant::now() + Duration::from_secs(3)
    }
    fn response() -> Vec<u8> {
        b"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc".to_vec()
    }
    fn clean_headers(bytes: &[u8]) {
        let text = std::str::from_utf8(bytes).unwrap().to_ascii_lowercase();
        assert!(text.starts_with("get /download?"));
        for field in [
            "authorization:",
            "proxy-authorization:",
            "cookie:",
            "referer:",
        ] {
            assert!(!text.contains(field));
        }
    }
    #[test]
    fn dedicated_tls_transport_validates_root_and_sends_no_backend_credentials() {
        let (address, server) = service(response(), Duration::ZERO);
        let client = client(address, "gchat.qpic.cn", true);
        let mut content = fetch(
            &client,
            URL,
            &crate::media_source::image_identity(URL).unwrap(),
            until(),
        )
        .unwrap();
        assert_eq!(content.size, Some(3));
        let mut bytes = Vec::new();
        content.reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        clean_headers(&server.join().unwrap());
    }
    #[test]
    fn tls_rejects_untrusted_roots_and_wrong_host_without_exposing_source() {
        for trusted in [false, true] {
            let (address, server) = service(response(), Duration::ZERO);
            let host = if trusted {
                "multimedia.nt.qq.com.cn"
            } else {
                "gchat.qpic.cn"
            };
            let url = URL.replace("gchat.qpic.cn", host);
            let client = client(address, host, trusted);
            let error = fetch(
                &client,
                &url,
                &crate::media_source::image_identity(&url).unwrap(),
                until(),
            )
            .err()
            .unwrap();
            assert_eq!(error.code, "MEDIA_TRANSPORT");
            let output = serde_json::to_string(&error).unwrap();
            assert!(!output.contains("private-key"));
            assert!(!output.contains(host));
            assert!(server.join().unwrap().is_empty());
        }
    }
    #[test]
    fn image_http_rejects_redirects_encoding_type_size_and_obeys_deadline() {
        for (headers, code, delay) in [
            (
                "302 Found\r\nLocation: https://invalid.invalid/forbidden\r\nContent-Length: 0",
                "MEDIA_HTTP_REJECTED",
                Duration::ZERO,
            ),
            (
                "403 Forbidden\r\nContent-Length: 0",
                "MEDIA_HTTP_REJECTED",
                Duration::ZERO,
            ),
            (
                "200 OK\r\nContent-Type: text/html\r\nContent-Length: 3",
                "MEDIA_HTTP_INVALID",
                Duration::ZERO,
            ),
            (
                "200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 3",
                "MEDIA_HTTP_INVALID",
                Duration::ZERO,
            ),
            (
                "200 OK\r\nContent-Length: 134217729",
                "MEDIA_HTTP_INVALID",
                Duration::ZERO,
            ),
            (
                "200 OK\r\nContent-Length: 3",
                "MEDIA_TRANSPORT",
                Duration::from_millis(250),
            ),
        ] {
            let (address, server) = service(
                format!("HTTP/1.1 {headers}\r\nConnection: close\r\n\r\nabc").into_bytes(),
                delay,
            );
            let client = client(address, "gchat.qpic.cn", true);
            let deadline = if delay.is_zero() {
                until()
            } else {
                Instant::now() + Duration::from_millis(100)
            };
            let error = fetch(
                &client,
                URL,
                &crate::media_source::image_identity(URL).unwrap(),
                deadline,
            )
            .err()
            .unwrap();
            assert_eq!(error.code, code);
            let request = server.join().unwrap();
            // A cold certificate check may consume the deadline before HTTP is sent.
            if delay.is_zero() || !request.is_empty() {
                clean_headers(&request);
            }
        }
    }
    #[test]
    fn scoped_image_download_streams_tls_bytes_and_withholds_output_after_account_change() {
        use crate::{
            client::{Action, Backend},
            config::Config,
            domain::{Conversation, Scope},
            references::References,
            session::Session,
            state::Instance,
        };
        use serde_json::{Value, json};
        use std::cell::Cell;
        struct Http(Client);
        impl Images for Http {
            fn open(&self, url: &str, identity: &str, deadline: Instant) -> Result<Content> {
                fetch(&self.0, url, identity, deadline)
            }
        }
        struct Fake {
            raw: Value,
            identities: Cell<usize>,
            guard: bool,
        }
        impl Backend for Fake {
            fn call(&self, action: Action, _: &Value, _: Instant) -> Result<Value> {
                Ok(match action {
                    Action::VersionInfo => json!({"app_version":"4.18.38"}),
                    Action::Status => json!({"online":true}),
                    Action::LoginInfo => {
                        self.identities.set(self.identities.get() + 1);
                        json!({"user_id":if self.guard && self.identities.get()>=5 {"10003"}else{"10001"}})
                    }
                    Action::Message => self.raw.clone(),
                    _ => panic!("Unexpected synthetic image action"),
                })
            }
        }
        for private in [false, true] {
            for guard in [false, true] {
                let temp = tempfile::tempdir().unwrap();
                let refs = References::with_key(temp.path().join("refs"), [93; 32]);
                let mut raw = json!({"message_id":"40000","real_seq":"100","time":1700000000,"self_id":"10001","user_id":"10002","message_type":if private {"private"}else{"group"},"message":[{"type":"image","data":{"file":"same.png","file_size":"3","url":URL}}]});
                raw[if private { "target_id" } else { "group_id" }] =
                    json!(if private { "10001" } else { "20001" });
                let scope = Scope {
                    instance_id: "instance-example".into(),
                    account: "10001".into(),
                };
                // History omits image URLs to avoid eagerly resolving every CDN key.
                // Download must recover the source from the same fresh message element.
                let mut historic = raw.clone();
                if private {
                    historic["message"][0]["data"]
                        .as_object_mut()
                        .unwrap()
                        .remove("url");
                } else {
                    historic["message"][0]["data"]["url"] = json!("synthetic-cache/image.png");
                }
                let message = crate::message::normalize(
                    &historic,
                    &scope,
                    "run-example",
                    &if private {
                        Conversation::Private("10002".into())
                    } else {
                        Conversation::Group("20001".into())
                    },
                    &refs,
                )
                .unwrap();
                assert_eq!(message.value["segments"][0]["downloadable"], true);
                let token = message.value["segments"][0]["mediaRef"].as_str().unwrap();
                let instance = Instance {
                    id: "instance-example".into(),
                    installation_id: "install-example".into(),
                    endpoint: "http://127.0.0.1:1/".into(),
                    ws_endpoint: None,
                    credential_key: "unused".into(),
                    account: None,
                    run_id: "run-example".into(),
                    state: "ready".into(),
                    process_id: None,
                    process_started: None,
                };
                let config = Config::default();
                let backend = Fake {
                    raw,
                    identities: Cell::new(0),
                    guard,
                };
                let session = Session {
                    backend: &backend,
                    config: &config,
                    instance: &instance,
                    expected_version: "4.18.38",
                };
                let (address, server) = service(response(), Duration::ZERO);
                let http = Http(client(address, "gchat.qpic.cn", true));
                let output = temp.path().join("image.png");
                let result = crate::media::download_with_images(
                    token,
                    &output,
                    &session,
                    &refs,
                    until(),
                    &http,
                );
                if guard {
                    assert_eq!(result.err().unwrap().code, "ACCOUNT_CHANGED");
                    assert!(!output.exists());
                } else {
                    let result = serde_json::to_value(result.unwrap()).unwrap();
                    assert_eq!(std::fs::read(output).unwrap(), b"abc");
                    assert_eq!(result["meta"]["scope"]["account"], "10001");
                    assert_eq!(result["data"]["kind"], "image");
                    let schema = crate::registry::get("media.download").unwrap();
                    assert!(
                        jsonschema::validator_for(&schema["responseSchema"])
                            .unwrap()
                            .is_valid(&result)
                    );
                }
                clean_headers(&server.join().unwrap());
            }
        }
    }
}

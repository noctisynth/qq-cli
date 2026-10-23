//! Private shared POST transport over verified sockets; protocol wrappers own routes and credentials.
use super::ownership::Guard;
use crate::core::error::{AppError, Result};
use crate::storage::config::Config;
use crate::transport::client::Endpoint;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::header::{
    ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONNECTION, CONTENT_ENCODING, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, HeaderValue, TRANSFER_ENCODING,
};
use hyper_util::rt::TokioIo;
use serde_json::Value;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};
const REQUEST_LIMIT: usize = 4 * 1024 * 1024;
pub(crate) struct OwnedHttp<'a> {
    guard: Guard<'a>,
    address: SocketAddr,
    authority: String,
    timeout: Duration,
    runtime: tokio::runtime::Runtime,
}
impl<'a> OwnedHttp<'a> {
    /// Caller obtains this job/process from authenticated lifecycle state and completes startup gates.
    /// No network or credentials are sent by construction. This synchronous backend is not reentrant.
    pub fn new(guard: Guard<'a>, endpoint: &str, config: &Config) -> Result<Self> {
        let endpoint = Endpoint::parse(endpoint)?;
        if !(1024..=REQUEST_LIMIT).contains(&config.response_bytes)
            || !(1..=60).contains(&config.request_timeout_seconds)
        {
            return Err(AppError::new(
                "CONFIG_INVALID",
                "Backend transport limits are invalid.",
                2,
            ));
        }
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(context());
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|_| context())?;
        let authority = endpoint
            .as_str()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        let address = authority.parse().map_err(|_| AppError::invalid())?;
        Ok(Self {
            guard,
            address,
            authority,
            timeout: Duration::from_secs(config.request_timeout_seconds.into()),
            runtime,
        })
    }
    pub(crate) fn post(
        &self,
        path: &str,
        credential: Option<&str>,
        payload: &Value,
        deadline: Instant,
        limit: usize,
    ) -> Result<Vec<u8>> {
        crate::core::deadline::check(deadline)?;
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(context());
        }
        if !path.starts_with('/')
            || path.contains("//")
            || !path
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_'))
        {
            return Err(AppError::invalid());
        }
        let authorization = credential.map(authorization).transpose()?;
        let deadline = deadline.min(Instant::now() + self.timeout);
        let mut body = Payload(Vec::new());
        serde_json::to_writer(&mut body, payload).map_err(|_| request_limit())?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(timeout)?;
        let stream =
            TcpStream::connect_timeout(&self.address, remaining.min(Duration::from_secs(5)))
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::ConnectionRefused {
                        AppError::new(
                            "BACKEND_NOT_LISTENING",
                            "Backend refused the connection before any request was sent.",
                            3,
                        )
                    } else {
                        transport()
                    }
                })?;
        let stream = self.guard.bind(stream, deadline)?;
        stream.set_nonblocking(true).map_err(|_| transport())?;
        // Authorization is constructed into a request only AFTER this exact socket's ownership proof.
        let mut request = hyper::Request::builder()
            .method("POST")
            .uri(path)
            .header(HOST, &self.authority)
            .header(CONNECTION, "close")
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .header(ACCEPT_ENCODING, "identity")
            .header(CONTENT_LENGTH, body.0.len());
        if let Some(authorization) = authorization {
            request = request.header(AUTHORIZATION, authorization);
        }
        let request = request
            .body(Full::new(Bytes::from(body.0)))
            .map_err(|_| AppError::invalid())?;
        let response = self.runtime.block_on(async {
            tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), async {
                crate::core::deadline::check(deadline)?;
                let io = tokio::net::TcpStream::from_std(stream).map_err(|_| transport())?;
                let mut builder = hyper::client::conn::http1::Builder::new();
                builder
                    .http09_responses(false)
                    .allow_obsolete_multiline_headers_in_responses(false)
                    .allow_spaces_after_header_name_in_responses(false)
                    .ignore_invalid_headers_in_responses(false)
                    .max_buf_size(32 * 1024)
                    .max_header_size(32 * 1024)
                    .max_headers(64);
                let (mut sender, connection) = builder
                    .handshake(TokioIo::new(io))
                    .await
                    .map_err(|_| transport())?;
                let mut task = ConnectionTask(tokio::spawn(connection));
                let result = async {
                    crate::core::deadline::check(deadline)?;
                    let response = sender
                        .send_request(request)
                        .await
                        .map_err(|_| transport())?;
                    let status = response.status().as_u16();
                    if matches!(status, 401 | 403) {
                        return Err(AppError::new(
                            "BACKEND_AUTH",
                            "Backend rejected authentication.",
                            3,
                        ));
                    }
                    if !(200..300).contains(&status) {
                        return Err(AppError::new(
                            "BACKEND_HTTP",
                            "Backend returned an unsuccessful HTTP status.",
                            3,
                        ));
                    }
                    for value in response.headers().get_all(CONTENT_ENCODING) {
                        if !value
                            .to_str()
                            .is_ok_and(|s| s.eq_ignore_ascii_case("identity"))
                        {
                            return Err(AppError::protocol());
                        }
                    }
                    for value in response.headers().get_all(TRANSFER_ENCODING) {
                        if !value
                            .to_str()
                            .is_ok_and(|s| s.eq_ignore_ascii_case("chunked"))
                        {
                            return Err(AppError::protocol());
                        }
                    }
                    for value in response.headers().get_all(CONTENT_LENGTH) {
                        let size = value
                            .to_str()
                            .ok()
                            .and_then(|s| s.parse::<u64>().ok())
                            .ok_or_else(AppError::protocol)?;
                        if size > limit as u64 {
                            return Err(response_limit());
                        }
                    }
                    let mut body = response.into_body();
                    let mut bytes = Vec::new();
                    while let Some(frame) = body.frame().await {
                        let frame = frame.map_err(|_| transport())?;
                        if let Ok(data) = frame.into_data() {
                            if data.len() > limit.saturating_sub(bytes.len()) {
                                return Err(response_limit());
                            }
                            bytes.extend_from_slice(&data);
                        }
                    }
                    Ok(bytes)
                }
                .await;
                task.0.abort();
                let _ = (&mut task.0).await;
                result
            })
            .await
            .map_err(|_| timeout())?
        })?;
        self.guard.verify(deadline)?;
        crate::core::deadline::check(deadline)?;
        Ok(response)
    }
}
struct Payload(Vec<u8>);
impl Write for Payload {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > REQUEST_LIMIT.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("request limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct ConnectionTask(tokio::task::JoinHandle<std::result::Result<(), hyper::Error>>);
impl Drop for ConnectionTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
fn context() -> AppError {
    AppError::new(
        "HTTP_RUNTIME_CONTEXT",
        "Owned backend requires a synchronous runtime context.",
        6,
    )
}
fn transport() -> AppError {
    AppError::new(
        "BACKEND_TRANSPORT",
        "Backend request or response failed; no automatic retry was attempted.",
        3,
    )
}
fn timeout() -> AppError {
    AppError::new("DEADLINE", "Operation deadline exceeded.", 3)
}
fn request_limit() -> AppError {
    AppError::new(
        "REQUEST_LIMIT",
        "Backend request exceeds its size limit.",
        2,
    )
}
fn response_limit() -> AppError {
    AppError::new(
        "RESPONSE_LIMIT",
        "Backend response exceeds its size limit.",
        3,
    )
}

pub(crate) fn authorization(token: &str) -> Result<HeaderValue> {
    if token.is_empty() || token.len() > 4096 || !token.bytes().all(|b| (33..127).contains(&b)) {
        return Err(AppError::new(
            "CREDENTIAL_INVALID",
            "Backend credentials are invalid.",
            2,
        ));
    }
    let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| AppError::new("CREDENTIAL_INVALID", "Backend credentials are invalid.", 2))?;
    authorization.set_sensitive(true);
    Ok(authorization)
}

//! Forward WebSocket transport restricted to literal loopback sockets.
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use tungstenite::client::IntoClientRequest;
use tungstenite::http::header::{AUTHORIZATION, HeaderValue};
use tungstenite::protocol::{Message, WebSocket, WebSocketConfig};

use crate::client::Endpoint;
use crate::error::{AppError, Result};

pub enum Frame {
    Text(Vec<u8>),
    Control,
    Idle,
    Closed,
}

pub struct Connection {
    socket: WebSocket<BoundedSocket>,
    frames: usize,
}

pub fn validate_endpoint(input: &str) -> Result<(String, SocketAddr)> {
    let invalid = || {
        AppError::new(
            "WS_ENDPOINT_INVALID",
            "Event transport requires an explicit-port literal loopback ws endpoint with root or event path.",
            2,
        )
    };
    let rest = input.strip_prefix("ws://").ok_or_else(invalid)?;
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    if !matches!(path, "" | "event" | "event/") {
        return Err(invalid());
    }
    let endpoint = Endpoint::parse(&format!("http://{authority}")).map_err(|_| invalid())?;
    let authority = endpoint
        .as_str()
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let address = authority.parse().map_err(|_| invalid())?;
    let path = if path.is_empty() { "/" } else { "/event" };
    Ok((format!("ws://{authority}{path}"), address))
}

impl Connection {
    pub fn connect(endpoint: &str, token: &str, deadline: Instant) -> Result<Self> {
        Self::connect_with(endpoint, token, deadline, Ok)
    }

    /// Lifecycle must authenticate these native capabilities before calling.
    /// No Upgrade or Bearer bytes are sent until this exact socket has passed ownership verification.
    #[cfg(windows)]
    pub fn connect_owned(
        endpoint: &str,
        token: &str,
        job: &crate::native_job::Job,
        owner: &crate::process::Pinned,
        deadline: Instant,
    ) -> Result<Self> {
        let connection = Self::connect_with(endpoint, token, deadline, |stream| {
            job.bind_connected(stream, owner, deadline)
                .map(crate::native_job::BoundStream::into_stream)
        })?;
        owner.verify(deadline)?;
        if !job.contains(owner, deadline)? {
            return Err(AppError::new(
                "ENDPOINT_OWNER_MISMATCH",
                "Event endpoint process no longer matches its run.",
                4,
            ));
        }
        Ok(connection)
    }

    fn connect_with(
        endpoint: &str,
        token: &str,
        deadline: Instant,
        verify: impl FnOnce(TcpStream) -> Result<TcpStream>,
    ) -> Result<Self> {
        let (url, address) = validate_endpoint(endpoint)?;
        if token.is_empty() || token.len() > 4096 {
            return Err(credential());
        }
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| credential())?;
        authorization.set_sensitive(true);
        let mut request = url.into_client_request().map_err(|_| transport())?;
        request.headers_mut().insert(AUTHORIZATION, authorization);
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(expired)?;
        let stream = TcpStream::connect_timeout(&address, remaining.min(Duration::from_secs(5)))
            .map_err(|_| transport())?;
        let stream = verify(stream)?;
        crate::deadline::check(deadline)?;
        let stream = BoundedSocket {
            stream,
            deadline,
            idle: Duration::from_secs(10),
            received: 0,
            written: 0,
            receive_limit: 32 * 1024,
        };
        let config = WebSocketConfig::default()
            .read_buffer_size(16 * 1024)
            .write_buffer_size(0)
            .max_write_buffer_size(64 * 1024)
            .max_message_size(Some(4 * 1024 * 1024))
            .max_frame_size(Some(4 * 1024 * 1024));
        let (mut socket,_) = tungstenite::client::client_with_config(request,stream,Some(config)).map_err(|error| {
            if let tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response)) = error {
                if matches!(response.status().as_u16(),401|403) { return AppError::new("BACKEND_AUTH", "Event endpoint refused authentication.", 3); }
                return AppError::new("WS_HANDSHAKE", "Event endpoint did not accept a WebSocket upgrade; redirects are not followed.",3);
            }
            transport()
        })?;
        socket.get_mut().idle = Duration::from_millis(500);
        socket.get_mut().receive_limit = 64 * 1024 * 1024;
        Ok(Self { socket, frames: 0 })
    }

    pub fn receive(&mut self) -> Result<Frame> {
        if Instant::now() >= self.socket.get_ref().deadline {
            return Err(expired());
        }
        if self.frames >= 10_000 {
            return Err(AppError::new(
                "EVENT_STREAM_LIMIT",
                "Event transport reached its frame budget.",
                4,
            ));
        }
        let message = match self.socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return if Instant::now() >= self.socket.get_ref().deadline {
                    Err(expired())
                } else {
                    Ok(Frame::Idle)
                };
            }
            Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                return Ok(Frame::Closed);
            }
            Err(tungstenite::Error::Capacity(_)) => {
                return Err(AppError::new(
                    "EVENT_STREAM_LIMIT",
                    "Event transport exceeded its message size budget.",
                    4,
                ));
            }
            Err(_) => return Err(transport()),
        };
        self.frames += 1;
        match message {
            Message::Text(text) => Ok(Frame::Text(text.as_bytes().to_vec())),
            Message::Ping(_) => {
                self.socket.flush().map_err(|_| transport())?;
                Ok(Frame::Control)
            }
            Message::Pong(_) => Ok(Frame::Control),
            Message::Close(_) => {
                let _ = self.socket.flush();
                Ok(Frame::Closed)
            }
            _ => Err(AppError::new(
                "WS_PROTOCOL",
                "Event endpoint supplied a non-text data message.",
                3,
            )),
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _ = self.socket.get_mut().stream.shutdown(Shutdown::Both);
    }
}

struct BoundedSocket {
    stream: TcpStream,
    deadline: Instant,
    idle: Duration,
    received: usize,
    written: usize,
    receive_limit: usize,
}
impl BoundedSocket {
    fn timeout(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .map(|d| d.min(self.idle))
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "deadline"))
    }
}
impl Read for BoundedSocket {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.timeout()?))?;
        if self.received >= self.receive_limit {
            return Err(io::Error::other("receive budget"));
        }
        let size = buffer.len().min(self.receive_limit - self.received);
        let count = self.stream.read(&mut buffer[..size])?;
        self.received += count;
        Ok(count)
    }
}
impl Write for BoundedSocket {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.timeout()?))?;
        if self.written.saturating_add(buffer.len()) > 64 * 1024 {
            return Err(io::Error::other("write budget"));
        }
        let count = self.stream.write(buffer)?;
        self.written += count;
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.set_write_timeout(Some(self.timeout()?))?;
        self.stream.flush()
    }
}
fn credential() -> AppError {
    AppError::new("CREDENTIAL_INVALID", "Backend credentials are invalid.", 2)
}
fn transport() -> AppError {
    AppError::new(
        "WS_TRANSPORT",
        "Event connection or handshake was interrupted.",
        3,
    )
}
fn expired() -> AppError {
    AppError::new(
        "DEADLINE",
        "Event transport reached its operation deadline.",
        3,
    )
}

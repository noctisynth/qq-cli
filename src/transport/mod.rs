//! OneBot protocol and owned HTTP/WebSocket connections; no CLI routing.
pub mod client;
pub mod media;
#[cfg(windows)]
pub mod owned_http;
pub mod websocket;

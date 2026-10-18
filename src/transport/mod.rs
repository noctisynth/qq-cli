//! OneBot protocol and owned HTTP/WebSocket connections; no CLI routing.
pub mod client;
pub mod media;
#[cfg(windows)]
pub mod owned_http;
#[cfg(windows)]
pub(crate) mod owned_wire;
pub mod websocket;

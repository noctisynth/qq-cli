//! OneBot protocol and owned HTTP/WebSocket connections; no CLI routing.
pub mod catalog;
pub mod client;
pub mod media;
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub mod owned_http;
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) mod owned_wire;
pub mod websocket;

#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) mod ownership;

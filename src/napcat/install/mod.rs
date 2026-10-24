//! Fixed artifact acquisition, validation and installation ownership.
pub mod acquire;
pub mod archive;
pub mod catalog;
pub mod installation;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub mod macos;

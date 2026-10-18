//! Host observations and narrow Windows process/Job capabilities.
#[cfg(windows)]
#[allow(unsafe_code)] // Audited Win32 boundary; sibling modules remain checked.
pub mod job;
pub mod platform;
pub mod process;

//! Host observations and narrow Windows process/Job capabilities.
#[cfg(windows)]
#[path = "windows/credential_lock/mod.rs"]
#[allow(unsafe_code)] // Audited synchronization boundary; never receives credential values.
pub(crate) mod credential_lock;
#[cfg(windows)]
#[path = "windows/job/mod.rs"]
#[allow(unsafe_code)] // Audited Win32 boundary; sibling modules remain checked.
pub mod job;
pub mod platform;
pub mod process;

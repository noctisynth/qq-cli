//! Host observations and narrow platform process/connection capabilities.
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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/process.rs"]
#[allow(unsafe_code)] // Read-only libproc/kqueue boundary.
pub(crate) mod macos_process;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/socket.rs"]
#[allow(unsafe_code)] // Bounded descriptor queries and original SDK adapter only.
pub mod macos_socket;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/terminate.rs"]
#[allow(unsafe_code)] // SDK audit-token signalling; no numeric-PID fallback.
pub mod macos_terminate;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/signature.rs"]
pub mod macos_signature;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/signing_info.rs"]
#[allow(unsafe_code)] // Read-only CF signing dictionary, checked types and retained ownership.
pub(crate) mod macos_signing_info;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/acl.rs"]
#[allow(unsafe_code)] // SDK ACL snapshot/removal on a held, ownership-checked descriptor.
pub(crate) mod macos_acl;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/xattr.rs"]
#[allow(unsafe_code)] // Descriptor-only reads with compression metadata exposed.
pub(crate) mod macos_xattr;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/copy.rs"]
#[allow(unsafe_code)] // Held-descriptor copy and synchronous deadline callback only.
pub(crate) mod macos_copy;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "macos/copy/worker.rs"]
pub mod macos_copy_worker;

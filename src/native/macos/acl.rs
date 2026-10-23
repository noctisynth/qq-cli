//! Descriptor ACL snapshots and explicit removal on an owned private copy.
use crate::core::error::{AppError, Result};
use sha2::{Digest, Sha256};
use std::ffi::c_void;
use std::os::fd::{AsRawFd, BorrowedFd};

unsafe extern "C" {
    fn acl_get_fd_np(fd: i32, kind: u32) -> *mut c_void;
    fn acl_size(acl: *mut c_void) -> isize;
    fn acl_copy_ext(buffer: *mut c_void, acl: *mut c_void, size: isize) -> isize;
    fn acl_free(acl: *mut c_void) -> i32;
    fn acl_init(count: i32) -> *mut c_void;
    fn acl_set_fd_np(fd: i32, acl: *mut c_void, kind: u32) -> i32;
}

struct Acl(*mut c_void);
impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: sole owner of a non-null allocation returned by acl_get_fd_np.
        unsafe { acl_free(self.0) };
    }
}

/// Snapshot the ACL on the held descriptor, including an O_SYMLINK descriptor.
/// This proves neither private access nor permission to execute the object.
pub(crate) fn digest(file: BorrowedFd<'_>) -> Result<[u8; 32]> {
    // SAFETY: borrowed descriptor stays live; ACL_TYPE_EXTENDED is Darwin's
    // documented 0x100 value. The SDK returns an independently owned snapshot.
    let pointer = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
    if pointer.is_null() {
        // Darwin returns ENOENT when this held object's extended ACL is absent.
        // There is no pathname lookup here; other failures remain errors.
        if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            return Ok(absent_digest());
        }
        return Err(invalid());
    }
    let acl = Acl(pointer);
    // SAFETY: the owned ACL remains live throughout both SDK calls.
    let size = unsafe { acl_size(acl.0) };
    if !(1..=65536).contains(&size) {
        return Err(invalid());
    }
    let mut bytes = vec![0_u8; size as usize];
    // SAFETY: writable initialized allocation of exactly size bytes; the SDK
    // writes its big-endian external representation and retains no pointer.
    let written = unsafe { acl_copy_ext(bytes.as_mut_ptr().cast(), acl.0, size) };
    if written != size {
        return Err(invalid());
    }
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-darwin-acl-v1\0");
    hash.update(bytes);
    Ok(hash.finalize().into())
}

fn invalid() -> AppError {
    AppError::new(
        "MACOS_METADATA_UNVERIFIABLE",
        "Application ACL could not be verified or updated; no copy was accepted.",
        3,
    )
}

/// Private staging parents must not grant access through an extended ACL.
pub(crate) fn require_absent(file: BorrowedFd<'_>) -> Result<()> {
    if digest(file)? != absent_digest() {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) fn absent_digest() -> [u8; 32] {
    Sha256::digest(b"qq-cli-darwin-acl-absent-v1\0").into()
}

/// Caller must hold the capability for a verified private copy. This does not
/// authorize clearing ACLs on arbitrary paths or the original installation.
pub(crate) fn clear_owned(file: BorrowedFd<'_>) -> Result<()> {
    let meta = rustix::fs::fstat(file).map_err(|_| invalid())?;
    let kind = meta.st_mode & libc::S_IFMT;
    if meta.st_uid != rustix::process::geteuid().as_raw()
        || !matches!(kind, libc::S_IFREG | libc::S_IFDIR | libc::S_IFLNK)
        || (kind == libc::S_IFREG && meta.st_nlink != 1)
    {
        return Err(invalid());
    }
    // SAFETY: SDK allocates an independent empty ACL, owned below exactly once.
    let pointer = unsafe { acl_init(0) };
    if pointer.is_null() {
        return Err(invalid());
    }
    let acl = Acl(pointer);
    // SAFETY: held live descriptor and owned valid ACL. O_SYMLINK targets the
    // link itself; no path lookup or pointer retention occurs.
    if unsafe { acl_set_fd_np(file.as_raw_fd(), acl.0, 0x100) } != 0 {
        return Err(invalid());
    }
    require_absent(file)
}

//! Descriptor-only reads including Darwin's normally hidden compression metadata.
use std::ffi::CStr;
use std::io;
use std::os::fd::{AsRawFd, BorrowedFd};

pub(crate) fn list(file: BorrowedFd<'_>, buffer: &mut [u8]) -> io::Result<usize> {
    let pointer = if buffer.is_empty() {
        std::ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast()
    };
    // SAFETY: borrowed live descriptor and initialized writable slice of the
    // exact declared length. The SDK retains no pointer. No authorization bypass.
    let count = unsafe {
        libc::flistxattr(
            file.as_raw_fd(),
            pointer,
            buffer.len(),
            libc::XATTR_SHOWCOMPRESSION,
        )
    };
    result(count)
}

pub(crate) fn read(file: BorrowedFd<'_>, name: &CStr, buffer: &mut [u8]) -> io::Result<usize> {
    read_at(file, name, buffer, 0)
}

pub(crate) fn read_at(
    file: BorrowedFd<'_>,
    name: &CStr,
    buffer: &mut [u8],
    position: u32,
) -> io::Result<usize> {
    let pointer = if buffer.is_empty() {
        std::ptr::null_mut()
    } else {
        buffer.as_mut_ptr().cast()
    };
    // SAFETY: live borrowed descriptor, terminated name, bounded initialized
    // slice (or NULL for sizing); caller uses offsets only for ResourceFork.
    let count = unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            name.as_ptr(),
            pointer,
            buffer.len(),
            position,
            libc::XATTR_SHOWCOMPRESSION,
        )
    };
    result(count)
}

fn result(count: isize) -> io::Result<usize> {
    if count < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(count as usize)
    }
}

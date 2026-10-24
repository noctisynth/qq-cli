//! Original raw-device handles, not mount ownership. No force, format or path-based eject.
use crate::core::error::{AppError, Result};
use rustix::fs::{Mode, OFlags};
use std::{fs::File, os::fd::AsRawFd, path::Path, time::Instant};
unsafe extern "C" {
    fn qq_cli_disk_matches(fd: i32, major: u32, minor: u32) -> i32;
    fn qq_cli_disk_eject_once(fd: i32, major: u32, minor: u32) -> i32;
}
pub(crate) struct Disk {
    file: File,
    major: u32,
    minor: u32,
}
impl Disk {
    pub(crate) fn open(name: &str, major: u32, minor: u32) -> Result<Self> {
        let suffix = name.strip_prefix("/dev/disk").ok_or_else(invalid)?;
        if suffix.is_empty()
            || suffix.len() > 40
            || suffix
                .split('s')
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Err(invalid());
        }
        let raw = format!("/dev/rdisk{suffix}");
        crate::storage::state::reject_links(Path::new(&raw))?;
        let file = File::from(
            rustix::fs::open(
                raw.as_str(),
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        Self::from_file(file, major, minor)
    }
    pub(crate) fn from_file(file: File, major: u32, minor: u32) -> Result<Self> {
        if rustix::fs::fcntl_getfl(&file).map_err(|_| invalid())? & OFlags::ACCMODE
            != OFlags::RDONLY
        {
            return Err(invalid());
        }
        rustix::io::fcntl_setfd(&file, rustix::io::FdFlags::CLOEXEC).map_err(|_| invalid())?;
        let disk = Self { file, major, minor };
        disk.verify()?;
        Ok(disk)
    }
    pub(crate) fn into_parts(self) -> (File, (u32, u32)) {
        (self.file, (self.major, self.minor))
    }
    pub(crate) fn verify(&self) -> Result<()> {
        // SAFETY: a live borrowed descriptor and integers only; adapter retains nothing.
        if unsafe { qq_cli_disk_matches(self.file.as_raw_fd(), self.major, self.minor) } != 0 {
            return Err(invalid());
        }
        Ok(())
    }
    pub(crate) fn eject_once(
        self,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        self.verify()?;
        crate::core::deadline::check(deadline)?;
        // SAFETY: consumes the original open FD. The SDK adapter checks character
        // device identity and sends DKIOCEJECT once, never force/format or a new path.
        let error =
            unsafe { qq_cli_disk_eject_once(self.file.as_raw_fd(), self.major, self.minor) };
        if error != 0 {
            return Err(AppError::new(
                if error == libc::EBUSY {
                    "QQ_MEDIA_BUSY"
                } else {
                    "QQ_MEDIA_EJECT_REJECTED"
                },
                "The original disk rejected ejection; no force or alternate device was attempted.",
                4,
            ));
        }
        // Acceptance is not disappearance or journal completion proof.
        Ok(())
    }
}
fn invalid() -> AppError {
    AppError::new(
        "QQ_MEDIA_DEVICE_INVALID",
        "Original read-only disk device identity could not be pinned.",
        4,
    )
}

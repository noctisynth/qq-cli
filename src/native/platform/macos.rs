//! Global OS build metadata only; no user/device identifiers or subprocesses.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
};

pub(crate) fn system_version_digest() -> Result<String> {
    let path = Path::new("/System/Library/CoreServices/SystemVersion.plist");
    crate::storage::state::reject_links(path)?;
    let mut file: File = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| unverified())?;
    let before = file.metadata().map_err(|_| unverified())?;
    if !before.is_file() || before.len() == 0 || before.len() > 65536 {
        return Err(unverified());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| unverified())?;
    let after = file.metadata().map_err(|_| unverified())?;
    let current = std::fs::symlink_metadata(path).map_err(|_| unverified())?;
    if bytes.len() as u64 != before.len()
        || stamp(&before) != stamp(&after)
        || stamp(&before) != stamp(&current)
    {
        return Err(unverified());
    }
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn stamp(m: &Metadata) -> (u64, u64, u64, u32, u32, u32, i64, i64, i64, i64) {
    (
        m.dev(),
        m.ino(),
        m.len(),
        m.mode(),
        m.uid(),
        m.gid(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
fn unverified() -> AppError {
    AppError::new(
        "MACOS_VERSION_UNVERIFIED",
        "The original system version metadata could not be verified.",
        4,
    )
}

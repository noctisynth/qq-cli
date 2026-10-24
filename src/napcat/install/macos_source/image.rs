//! Fixed DMG byte observation, not a mount receipt or an application source.
pub mod acquire;
pub mod attach;
pub mod journal;
pub mod mounted;
mod quarantine;
pub mod release;
mod supervision;
pub mod transition;

use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;
use rustix::fs::{Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(PartialEq, Eq, serde::Serialize)]
struct Stamp {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
fn stamp(file: &File) -> Result<Stamp> {
    let m = file.metadata().map_err(|_| invalid())?;
    if !m.is_file() {
        return Err(invalid());
    }
    Ok(Stamp {
        device: m.dev(),
        inode: m.ino(),
        size: m.len(),
        modified: (m.mtime(), m.mtime_nsec()),
        changed: (m.ctime(), m.ctime_nsec()),
    })
}
fn open(path: &Path) -> Result<File> {
    reject_links(path)?;
    if !path.is_absolute() || path.canonicalize().map_err(|_| invalid())? != path {
        return Err(invalid());
    }
    rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|_| invalid())
}
/// Owns the exact opened input and compares it with the current path on each check.
/// Future mount dispatch must revalidate it at its own effect boundary.
pub struct Image {
    path: PathBuf,
    file: File,
    stamp: Stamp,
    digest: String,
    quarantine: Option<Vec<u8>>,
}
impl Image {
    pub fn inspect(path: &Path, deadline: Instant) -> Result<Self> {
        Self::inspect_expected(path, super::DMG_BYTES, super::DMG_SHA256, deadline)
    }
    fn inspect_expected(path: &Path, bytes: u64, digest: &str, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let file = open(path)?;
        let original = stamp(&file)?;
        let quarantine = quarantine::read(&file)?;
        if original.size != bytes {
            return Err(invalid());
        }
        let image = Self {
            path: path.into(),
            file,
            stamp: original,
            digest: digest.into(),
            quarantine,
        };
        image.revalidate(deadline)?;
        Ok(image)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn revalidate(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if stamp(&self.file)? != self.stamp
            || stamp(&open(&self.path)?)? != self.stamp
            || quarantine::read(&self.file)? != self.quarantine
        {
            return Err(invalid());
        }
        let mut hash = Sha256::new();
        let mut offset = 0u64;
        let mut buffer = [0u8; 65536];
        while offset < self.stamp.size {
            crate::core::deadline::check(deadline)?;
            let size = usize::try_from((self.stamp.size - offset).min(buffer.len() as u64))
                .map_err(|_| invalid())?;
            let count = rustix::io::pread(&self.file, &mut buffer[..size], offset)
                .map_err(|_| invalid())?;
            if count == 0 {
                return Err(invalid());
            }
            hash.update(&buffer[..count]);
            offset += count as u64;
        }
        if format!("{:x}", hash.finalize()) != self.digest
            || stamp(&self.file)? != self.stamp
            || stamp(&open(&self.path)?)? != self.stamp
            || quarantine::read(&self.file)? != self.quarantine
        {
            return Err(invalid());
        }
        crate::core::deadline::check(deadline)
    }
}
fn invalid() -> AppError {
    AppError::new(
        "QQ_IMAGE_INVALID",
        "QQ disk image bytes, metadata or original file identity do not match the fixed source.",
        4,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }
    #[test]
    fn pinned_bytes_and_original_path_are_both_required() {
        for change in 0..3 {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().canonicalize().unwrap().join("source.dmg");
            std::fs::write(&path, b"synthetic disk image").unwrap();
            let digest = format!("{:x}", Sha256::digest(b"synthetic disk image"));
            let image = Image::inspect_expected(
                &path,
                b"synthetic disk image".len() as u64,
                &digest,
                deadline(),
            )
            .unwrap();
            image.revalidate(deadline()).unwrap();
            match change {
                0 => std::fs::write(&path, b"changed disk image!!").unwrap(),
                1 => {
                    std::fs::rename(&path, path.with_extension("old")).unwrap();
                    std::fs::write(&path, b"synthetic disk image").unwrap();
                }
                _ => {
                    std::fs::rename(&path, path.with_extension("old")).unwrap();
                    std::os::unix::fs::symlink(path.with_extension("old"), &path).unwrap();
                }
            }
            assert!(image.revalidate(deadline()).is_err());
        }
    }
    #[test]
    fn wrong_digest_size_and_expiry_reject_without_mounting() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().canonicalize().unwrap().join("source.dmg");
        std::fs::write(&path, b"data").unwrap();
        assert!(Image::inspect_expected(&path, 4, &"0".repeat(64), deadline()).is_err());
        assert!(Image::inspect_expected(&path, 5, &"0".repeat(64), deadline()).is_err());
        assert!(Image::inspect(&path, Instant::now()).is_err());
        assert!(Image::inspect(&path, deadline()).is_err());
    }
}

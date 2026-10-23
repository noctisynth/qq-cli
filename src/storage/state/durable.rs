//! Descriptor-relative publication and durable ordering for macOS private JSON state.
use crate::core::error::{AppError, Result};
use rustix::fs::{AtFlags, Mode, OFlags, openat, renameat, statat, unlinkat};
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

#[path = "durable/locking.rs"]
mod locking;
pub(crate) use locking::Lock;

pub(crate) fn private_dir(path: &Path) -> Result<()> {
    let path = std::path::absolute(path).map_err(|_| AppError::storage())?;
    let mut directory = File::open("/").map_err(|_| AppError::storage())?;
    for component in path.components() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => name,
            _ => return Err(AppError::storage()),
        };
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let next = match openat(&directory, name, flags, Mode::empty()) {
            Ok(next) => next,
            Err(rustix::io::Errno::NOENT) => {
                match rustix::fs::mkdirat(&directory, name, Mode::RWXU) {
                    Ok(()) | Err(rustix::io::Errno::EXIST) => (),
                    Err(_) => return Err(AppError::storage()),
                }
                let next = openat(&directory, name, flags, Mode::empty())
                    .map_err(|_| AppError::storage())?;
                // Make ancestor links eligible for the later file-publication drive flush.
                directory.sync_all().map_err(|_| AppError::storage())?;
                next
            }
            Err(_) => return Err(AppError::storage()),
        };
        directory = File::from(next);
    }
    Directory {
        path,
        file: directory,
    }
    .verify()
}

pub(crate) struct Directory {
    path: PathBuf,
    file: File,
}
impl Directory {
    pub(crate) fn create_new(&self, name: &str) -> Result<File> {
        if !matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        ) {
            return Err(AppError::storage());
        }
        self.verify()?;
        openat(
            &self.file,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map(File::from)
        .map_err(|_| AppError::storage())
    }
    pub(crate) fn open(path: &Path) -> Result<Self> {
        let path = std::path::absolute(path).map_err(|_| AppError::storage())?;
        let mut directory = File::open("/").map_err(|_| AppError::storage())?;
        for component in path.components() {
            match component {
                Component::RootDir => (),
                Component::Normal(name) => {
                    directory = openat(
                        &directory,
                        name,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map(File::from)
                    .map_err(|_| AppError::storage())?;
                }
                _ => return Err(AppError::storage()),
            }
        }
        Ok(Self {
            path,
            file: directory,
        })
    }
    fn verify(&self) -> Result<()> {
        let current = Self::open(&self.path)?;
        let original = self.file.metadata().map_err(|_| AppError::storage())?;
        let current = current.file.metadata().map_err(|_| AppError::storage())?;
        if original.dev() != current.dev() || original.ino() != current.ino() {
            return Err(AppError::storage());
        }
        Ok(())
    }
    /// Directory publication precedes the drive-cache flush of the published file.
    pub(crate) fn sync_publication(&self, file: &File) -> Result<()> {
        self.file.sync_all().map_err(|_| AppError::storage())?;
        sync_file(file)?;
        self.verify()
    }
}
pub(crate) fn sync_file(file: &File) -> Result<()> {
    rustix::fs::fcntl_fullfsync(file).map_err(|_| AppError::storage())
}
struct Temporary<'a> {
    directory: &'a Directory,
    name: OsString,
    published: bool,
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        if !self.published {
            let _ = unlinkat(&self.directory.file, &self.name, AtFlags::empty());
        }
    }
}
pub(crate) fn atomic_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    super::reject_links(path)?;
    let directory = Directory::open(path.parent().ok_or_else(AppError::storage)?)?;
    let name = path.file_name().ok_or_else(AppError::storage)?;
    publish(&directory, name, value, || Ok(()))
}
fn publish(
    directory: &Directory,
    name: &std::ffi::OsStr,
    value: &impl serde::Serialize,
    before_publication: impl FnOnce() -> Result<()>,
) -> Result<()> {
    directory.verify()?;
    match statat(&directory.file, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) if stat.st_mode & libc::S_IFMT != libc::S_IFREG || stat.st_nlink != 1 => {
            return Err(AppError::storage());
        }
        Ok(_) | Err(rustix::io::Errno::NOENT) => (),
        Err(_) => return Err(AppError::storage()),
    }
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|_| AppError::storage())?;
    let temporary_name = format!(".qq-cli-{:032x}.tmp", u128::from_le_bytes(random));
    let mut file = openat(
        &directory.file,
        temporary_name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
    .map_err(|_| AppError::storage())?;
    let mut temporary = Temporary {
        directory,
        name: temporary_name.into(),
        published: false,
    };
    serde_json::to_writer(&mut file, value).map_err(|_| AppError::storage())?;
    file.write_all(b"\n").map_err(|_| AppError::storage())?;
    sync_file(&file)?;
    directory.verify()?;
    before_publication()?;
    renameat(&directory.file, &temporary.name, &directory.file, name)
        .map_err(|_| AppError::storage())?;
    temporary.published = true;
    directory.sync_publication(&file)
}

#[cfg(test)]
#[path = "durable/tests.rs"]
mod tests;

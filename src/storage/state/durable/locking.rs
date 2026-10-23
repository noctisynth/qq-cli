//! A lock retains its original directory and file identities, not just its open descriptor.
use super::*;
use std::fs::TryLockError;
use std::time::{Duration, Instant};

pub(crate) struct Lock {
    directory: Directory,
    name: String,
    file: File,
}
impl Lock {
    pub(crate) fn publish_registry(&self, value: &impl serde::Serialize) -> Result<()> {
        self.verify()?;
        super::publish(
            &self.directory,
            std::ffi::OsStr::new("registry.json"),
            value,
            || self.verify(),
        )?;
        self.verify()
    }
    pub(crate) fn acquire(
        root: &Path,
        name: &str,
        deadline: Instant,
        busy: &'static str,
    ) -> Result<Self> {
        if !matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        ) {
            return Err(AppError::storage());
        }
        let directory = Directory::open(root)?;
        check_directory(&directory)?;
        let flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        let opened = match openat(&directory.file, name, flags, Mode::empty()) {
            Ok(file) => Ok(file),
            Err(rustix::io::Errno::NOENT) => {
                match openat(
                    &directory.file,
                    name,
                    flags | OFlags::CREATE | OFlags::EXCL,
                    Mode::RUSR | Mode::WUSR,
                ) {
                    Ok(file) => Ok(file),
                    Err(rustix::io::Errno::EXIST) => {
                        openat(&directory.file, name, flags, Mode::empty())
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        let file = opened.map(File::from).map_err(|_| AppError::storage())?;
        let lock = Self {
            directory,
            name: name.into(),
            file,
        };
        loop {
            lock.verify()?;
            match lock.file.try_lock() {
                Ok(()) => {
                    crate::core::deadline::check(deadline)?;
                    lock.verify()?;
                    return Ok(lock);
                }
                Err(TryLockError::WouldBlock) => {
                    if Instant::now() >= deadline {
                        return Err(AppError::new(busy, "Another operation holds the original local lock; no replacement lock was adopted.", 6).safe_to_retry());
                    }
                    std::thread::sleep(
                        deadline
                            .saturating_duration_since(Instant::now())
                            .min(Duration::from_millis(10)),
                    );
                }
                Err(_) => return Err(AppError::storage()),
            }
        }
    }
    pub(crate) fn verify(&self) -> Result<()> {
        self.directory.verify()?;
        check_directory(&self.directory)?;
        let file = self.file.metadata().map_err(|_| AppError::storage())?;
        let current = statat(
            &self.directory.file,
            self.name.as_str(),
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(|_| AppError::storage())?;
        let uid = rustix::process::geteuid().as_raw();
        if !file.is_file()
            || file.uid() != uid
            || file.mode() & 0o022 != 0
            || file.nlink() != 1
            || current.st_mode & libc::S_IFMT != libc::S_IFREG
            || current.st_dev as u64 != file.dev()
            || current.st_ino != file.ino()
        {
            return Err(AppError::storage());
        }
        Ok(())
    }
}
fn check_directory(directory: &Directory) -> Result<()> {
    let metadata = directory.file.metadata().map_err(|_| AppError::storage())?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o022 != 0 {
        return Err(AppError::storage());
    }
    Ok(())
}

#[cfg(test)]
#[path = "locking/tests.rs"]
mod tests;

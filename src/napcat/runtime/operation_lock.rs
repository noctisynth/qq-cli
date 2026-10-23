//! Shared lifecycle lock; always acquired before short state transactions.
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use crate::core::error::AppError;
use crate::core::error::Result;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use crate::storage::state::reject_links;
use crate::storage::state::{Store, private_dir};
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use std::fs::{File, OpenOptions, TryLockError};
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use std::time::Duration;
use std::time::Instant;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
type Lock = File;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
type Lock = crate::storage::state::durable::Lock;
pub(crate) fn acquire(store: &Store, deadline: Instant) -> Result<Lock> {
    crate::core::deadline::check(deadline)?;
    private_dir(store.root())?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Lock::acquire(store.root(), "startup.lock", deadline, "START_BUSY")
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let path = store.root().join("startup.lock");
        reject_links(&path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| AppError::storage())?;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(AppError::new(
                    "START_BUSY",
                    "Another lifecycle operation is still active; no preparation or dispatch occurred.",
                    6,
                ).safe_to_retry());
                }
                Err(_) => return Err(AppError::storage()),
            }
        }
    }
}

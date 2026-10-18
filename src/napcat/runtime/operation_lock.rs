//! Shared lifecycle lock; always acquired before short state transactions.
use crate::core::error::{AppError, Result};
use crate::storage::state::{Store, private_dir, reject_links};
use std::fs::{File, OpenOptions, TryLockError};
use std::time::{Duration, Instant};
pub(crate) fn acquire(store: &Store, deadline: Instant) -> Result<File> {
    crate::core::deadline::check(deadline)?;
    private_dir(store.root())?;
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

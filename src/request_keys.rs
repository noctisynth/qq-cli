//! One key identifies one operation family across message writes and instance creation.
use crate::{
    error::{AppError, Result},
    state::{Store, reject_links},
};
use sha2::{Digest, Sha256};

fn exists(store: &Store, namespace: &str, key: &str) -> Result<bool> {
    crate::domain::request_key(key).map_err(|_| AppError::invalid())?;
    let path = store
        .root()
        .join(namespace)
        .join(format!("{:x}.json", Sha256::digest(key.as_bytes())));
    reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
pub fn has_start(store: &Store, key: &str) -> Result<bool> {
    Ok(exists(store, "instance-creations", key)? || exists(store, "start-requests", key)?)
}
/// Called while holding the state transaction, before creating any new write intent.
pub fn guard_message(store: &Store, key: &str) -> Result<()> {
    if has_start(store, key)? {
        Err(conflict())
    } else {
        Ok(())
    }
}
/// n1 and s1 belong to the same start operation and may share a key.
pub fn guard_start(store: &Store, key: &str) -> Result<()> {
    if exists(store, "requests", key)? {
        Err(conflict())
    } else {
        Ok(())
    }
}
pub fn conflict() -> AppError {
    AppError::new(
        "REQUEST_CONFLICT",
        "Request key is already occupied by a different operation; no result was selected or new side effect attempted.",
        4,
    )
}

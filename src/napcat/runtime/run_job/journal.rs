use super::{Outcome, Pointer, Record};
use crate::core::error::{AppError, Result};
use crate::storage::{
    config::read_json,
    state::{atomic_json, private_dir},
};
use crate::storage::{
    references::References,
    state::{Store, reject_links},
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub(super) fn path(store: &Store, instance: &str, run: &str) -> Result<PathBuf> {
    crate::core::domain::request_key(instance).map_err(|_| AppError::invalid())?;
    crate::core::domain::request_key(run).map_err(|_| AppError::invalid())?;
    let digest =
        Sha256::digest(serde_json::to_vec(&(instance, run)).map_err(|_| AppError::storage())?);
    Ok(store
        .root()
        .join("run-jobs")
        .join(format!("{digest:x}.json")))
}
/// Presence is only a routing signal, never validation or permission to access an API.
pub fn has_record(store: &Store, instance: &str, run: &str) -> Result<bool> {
    let path = path(store, instance, run)?;
    reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
pub(super) fn save(path: &Path, refs: &References, record: &Record) -> Result<()> {
    reject_links(path)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    let reference = refs.issue("j1", record)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}
pub(super) fn load(path: &Path, refs: &References) -> Result<Record> {
    reject_links(path)?;
    let pointer: Pointer = read_json(path, 4096)?;
    if pointer.schema_version != 1 {
        return Err(AppError::storage());
    }
    let record: Record = refs.get("j1", &pointer.reference)?;
    record.job.validate()?;
    crate::core::domain::request_key(&record.request_id).map_err(|_| AppError::storage())?;
    if record.schema_version != 1
        || (record.outcome == Outcome::Spawned) != record.root.is_some()
        || ((record.service.is_some() || record.prelogin.is_some())
            && record.outcome != Outcome::Spawned)
    {
        return Err(AppError::storage());
    }
    Ok(record)
}

/// A job capability comes only from a completed original dispatch, never a caller PID.
pub(crate) fn owned_job_id(
    store: &Store,
    refs: &References,
    instance: &crate::storage::state::Instance,
    installation: &crate::storage::state::Installation,
) -> Result<crate::native::job::JobId> {
    let record = load(&path(store, &instance.id, &instance.run_id)?, refs)?;
    if record.outcome != Outcome::Spawned
        || record.target.instance_id != instance.id
        || record.target.run_id != instance.run_id
        || instance.installation_id != installation.id
        || record.target.installation_id != installation.id
        || record.target.installation_digest != installation.digest
    {
        return Err(super::conflict());
    }
    Ok(record.job)
}

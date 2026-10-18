use crate::{
    core::error::{AppError, Result},
    napcat::runtime::runtime_store::Plan,
    storage::{
        references::References,
        state::{Installation, Instance, Store, atomic_json, private_dir, reject_links},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Record {
    pub schema_version: u8,
    pub source: Instance,
    pub installation: Installation,
    pub plan: Plan,
    pub root_run_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub(super) fn key(id: &str, run: &str) -> Result<String> {
    for part in [id, run] {
        crate::core::domain::request_key(part).map_err(|_| AppError::storage())?;
    }
    Ok(format!(
        "run-start-{:x}",
        Sha256::digest(serde_json::to_vec(&(id, run)).map_err(|_| AppError::storage())?)
    ))
}
pub(super) fn source_key(id: &str, run: &str) -> Result<String> {
    key(id, run)
}
fn path(store: &Store, origin: bool, key: &str) -> Result<std::path::PathBuf> {
    crate::core::domain::request_key(key).map_err(|_| AppError::storage())?;
    let path = store
        .root()
        .join(if origin {
            "run-origins"
        } else {
            "run-transitions"
        })
        .join(format!("{:x}.json", Sha256::digest(key.as_bytes())));
    reject_links(&path)?;
    Ok(path)
}
pub(super) fn exists(store: &Store, key: &str) -> Result<bool> {
    path(store, true, key)?
        .try_exists()
        .map_err(|_| AppError::storage())
}
pub(super) fn load(
    store: &Store,
    refs: &References,
    origin: bool,
    key: &str,
) -> Result<Option<Record>> {
    let pointer: Pointer = match crate::storage::config::read_json(&path(store, origin, key)?, 4096)
    {
        Ok(p) => p,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.schema_version != 1 {
        return Err(super::conflict());
    }
    let record: Record = refs.get("g1", &pointer.reference)?;
    for part in [
        &record.source.id,
        &record.source.run_id,
        &record.plan.run_id,
        &record.root_run_id,
        &record.installation.id,
    ] {
        crate::core::domain::request_key(part).map_err(|_| super::conflict())?;
    }
    record.plan.ports.validate()?;
    if record.schema_version != 1
        || record.source.state != "stopped"
        || record.plan.instance_id != record.source.id
        || record.plan.run_id == record.source.run_id
        || record.plan.run_id == record.root_run_id
        || record.installation.id != record.source.installation_id
        || record.plan.version.trim_start_matches('v')
            != record.installation.version.trim_start_matches('v')
        || record.installation.digest.len() != 64
        || !record
            .installation
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || (if origin {
            self::key(&record.plan.instance_id, &record.plan.run_id)?
        } else {
            source_key(&record.source.id, &record.source.run_id)?
        }) != key
    {
        return Err(super::conflict());
    }
    Ok(Some(record))
}
pub(super) fn save(
    store: &Store,
    refs: &References,
    origin: bool,
    key: &str,
    record: &Record,
) -> Result<()> {
    let path = path(store, origin, key)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference: refs.issue("g1", record)?,
        },
    )
}

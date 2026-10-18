use crate::{
    core::error::{AppError, Result},
    storage::{
        references::References,
        state::{Instance, Store, atomic_json, private_dir, reject_links},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Pending,
    Completed,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Record {
    pub schema_version: u8,
    pub instance: Instance,
    pub installation_digest: String,
    pub outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
fn path(store: &Store, id: &str) -> Result<std::path::PathBuf> {
    crate::core::domain::request_key(id).map_err(|_| AppError::invalid())?;
    let path = store
        .root()
        .join("instance-removals")
        .join(format!("{:x}.json", Sha256::digest(id.as_bytes())));
    reject_links(&path)?;
    Ok(path)
}
pub(super) fn load(store: &Store, refs: &References, id: &str) -> Result<Option<Record>> {
    let pointer: Pointer = match crate::storage::config::read_json(&path(store, id)?, 4096) {
        Ok(value) => value,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(AppError::storage());
    }
    let record: Record = refs.get("x1", &pointer.reference)?;
    if record.schema_version != 1
        || record.instance.id != id
        || record.instance.state != "stopped"
        || record.installation_digest.len() != 64
        || !record
            .installation_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AppError::storage());
    }
    for key in [&record.instance.run_id, &record.instance.installation_id] {
        crate::core::domain::request_key(key).map_err(|_| AppError::storage())?;
    }
    Ok(Some(record))
}
pub(super) fn has_record(store: &Store, id: &str) -> Result<bool> {
    path(store, id)?
        .try_exists()
        .map_err(|_| AppError::storage())
}
pub(super) fn save(store: &Store, refs: &References, record: &Record) -> Result<()> {
    let path = path(store, &record.instance.id)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference: refs.issue("x1", record)?,
        },
    )
}

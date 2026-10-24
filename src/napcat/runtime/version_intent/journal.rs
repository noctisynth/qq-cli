//! Fixed source index and signed records; no registry mutation or native effect.
use super::*;
use crate::storage::state::{atomic_json, private_dir, reject_links};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Update,
    Rollback,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Record {
    pub schema_version: u8,
    pub source: Instance,
    pub source_installation: Installation,
    pub target: Installation,
    pub operation: Operation,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

fn path(store: &Store, id: &str, run: &str) -> Result<std::path::PathBuf> {
    for key in [id, run] {
        crate::core::domain::request_key(key).map_err(|_| conflict())?;
    }
    let digest = Sha256::digest(serde_json::to_vec(&(id, run)).map_err(|_| conflict())?);
    let path = store
        .root()
        .join("version-maintenance")
        .join(format!("{digest:x}.json"));
    reject_links(&path)?;
    Ok(path)
}
pub(super) fn pointer(store: &Store, id: &str, run: &str) -> Result<Option<String>> {
    let pointer: Pointer = match crate::storage::config::read_json(&path(store, id, run)?, 4096) {
        Ok(value) => value,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(conflict());
    }
    Ok(Some(pointer.reference))
}
pub(super) fn load(store: &Store, refs: &References, reference: &str) -> Result<Intent> {
    let record: Record = refs.get(kind(), reference)?;
    validate(&record)?;
    if pointer(store, &record.source.id, &record.source.run_id)?.as_deref() != Some(reference) {
        return Err(conflict());
    }
    Ok(Intent {
        record,
        reference: reference.into(),
    })
}
fn validate(record: &Record) -> Result<()> {
    if record.schema_version != 1
        || record.source.installation_id != record.source_installation.id
        || record.source_installation.id == record.target.id
        || !matches!(
            record.source.state.as_str(),
            "ready" | "login_required" | "preparing" | "prepared" | "stopped" | "stopping"
        )
    {
        return Err(conflict());
    }
    for installation in [&record.source_installation, &record.target] {
        crate::core::domain::request_key(&installation.id).map_err(|_| conflict())?;
        if installation.digest.len() != 64
            || !installation
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(conflict());
        }
    }
    Ok(())
}
pub(super) fn save(
    store: &Store,
    refs: &References,
    record: Record,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Intent> {
    guard()?;
    validate(&record)?;
    let path = path(store, &record.source.id, &record.source.run_id)?;
    private_dir(path.parent().ok_or_else(conflict)?)?;
    let reference = refs.issue(kind(), &record)?;
    guard()?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference: reference.clone(),
        },
    )?;
    guard()?;
    Ok(Intent { record, reference })
}
fn kind() -> &'static str {
    if cfg!(windows) { "v1" } else { "v2" }
}

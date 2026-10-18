use crate::{
    core::error::{AppError, Result},
    native::job::JobId,
    storage::{
        references::References,
        state::{Store, atomic_json, private_dir, reject_links},
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Target {
    pub instance_id: String,
    pub run_id: String,
    pub installation_id: String,
    pub installation_digest: String,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Pending,
    Completed,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Record {
    pub schema_version: u8,
    pub target: Target,
    pub job: Option<JobId>,
    pub outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
fn path(store: &Store, target: &Target) -> Result<PathBuf> {
    for key in [&target.instance_id, &target.run_id, &target.installation_id] {
        crate::core::domain::request_key(key).map_err(|_| AppError::storage())?;
    }
    if target.installation_digest.len() != 64
        || !target
            .installation_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AppError::storage());
    }
    let digest = Sha256::digest(
        serde_json::to_vec(&(&target.instance_id, &target.run_id))
            .map_err(|_| AppError::storage())?,
    );
    let path = store
        .root()
        .join("instance-stops")
        .join(format!("{digest:x}.json"));
    reject_links(&path)?;
    Ok(path)
}
pub(super) fn load(store: &Store, refs: &References, target: &Target) -> Result<Option<Record>> {
    let path = path(store, target)?;
    let pointer: Pointer = match crate::storage::config::read_json(&path, 4096) {
        Ok(p) => p,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.schema_version != 1 {
        return Err(AppError::storage());
    }
    let record: Record = refs.get("p1", &pointer.reference)?;
    if record.schema_version != 1 || record.target != *target {
        return Err(AppError::storage());
    }
    if let Some(job) = &record.job {
        job.validate()?;
    }
    Ok(Some(record))
}
pub(super) fn save(store: &Store, refs: &References, record: &Record) -> Result<()> {
    let path = path(store, &record.target)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference: refs.issue("p1", record)?,
        },
    )
}

//! Signed local observations. Recording is restricted to the owned-run verifier.
use super::{EvidenceKind, Key, Verdict, Witness};
use crate::core::error::{AppError, Result};
use crate::napcat::runtime::start_intent::Target;
use crate::storage::{
    config::read_json,
    references::References,
    state::{Store, Transaction, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
const LIMIT: usize = 64;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    key: Key,
    source: Target,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

pub fn load(store: &Store, refs: &References) -> Result<Vec<Witness>> {
    let directory = store.root().join("compatibility-evidence");
    reject_links(&directory)?;
    if !directory.try_exists().map_err(|_| AppError::storage())? {
        return Ok(vec![]);
    }
    let mut witnesses = Vec::new();
    for entry in std::fs::read_dir(directory).map_err(|_| AppError::storage())? {
        if witnesses.len() >= LIMIT {
            return Err(AppError::storage());
        }
        let entry = entry.map_err(|_| AppError::storage())?;
        let path = entry.path();
        reject_links(&path)?;
        if !entry
            .file_type()
            .map_err(|_| AppError::storage())?
            .is_file()
        {
            return Err(AppError::storage());
        }
        let pointer: Pointer = read_json(&path, 4096)?;
        let record: Record = refs.get("k1", &pointer.reference)?;
        let fingerprint = record.key.fingerprint();
        if pointer.schema_version != 1
            || record.schema_version != 1
            || path.file_name().and_then(|n| n.to_str())
                != Some(format!("{fingerprint}.json").as_str())
            || record.source.installation_digest != record.key.archive_sha256
            || record.source.input_hash.len() != 64
            || !record
                .source
                .input_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            return Err(AppError::storage());
        }
        crate::core::domain::request_key(&record.source.instance_id)
            .map_err(|_| AppError::storage())?;
        crate::core::domain::request_key(&record.source.run_id).map_err(|_| AppError::storage())?;
        witnesses.push(Witness {
            key: record.key,
            verdict: Verdict::Supported,
            kind: EvidenceKind::FrameworkStartup,
            reference: format!("LOCAL_FRAMEWORK_STARTUP:{fingerprint}"),
        });
    }
    witnesses.sort_by(|a, b| a.reference.cmp(&b.reference));
    Ok(witnesses)
}

/// Caller has revalidated the launch hash, native service and online identity under tx.
pub(crate) fn record(
    tx: &Transaction<'_>,
    refs: &References,
    key: Key,
    source: Target,
) -> Result<()> {
    if source.installation_digest != key.archive_sha256 {
        return Err(AppError::storage());
    }
    let existing = load(tx.store(), refs)?;
    if existing.iter().any(|w| w.key == key) {
        return Ok(());
    }
    if existing.len() >= LIMIT {
        return Err(AppError::storage());
    }
    let directory = tx.store().root().join("compatibility-evidence");
    private_dir(&directory)?;
    let path = directory.join(format!("{}.json", key.fingerprint()));
    reject_links(&path)?;
    let reference = refs.issue(
        "k1",
        &Record {
            schema_version: 1,
            key,
            source,
        },
    )?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}

#[cfg(test)]
mod tests;

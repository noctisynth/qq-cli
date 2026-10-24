//! Signed local positive observations; only the original owned-run verifier writes them.
use super::*;
use crate::napcat::runtime::start_intent::Target;
use crate::storage::{
    config::read_json,
    references::References,
    state::{Store, Transaction, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
const LIMIT: usize = 64;
const DIRECTORY: &str = "macos-compatibility";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema_version: u8,
    reference: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u8,
    key: Key,
    source: Target,
}

pub(super) fn present(store: &Store) -> Result<bool> {
    let path = store.root().join(DIRECTORY);
    reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
fn records(store: &Store, refs: &References) -> Result<Vec<Record>> {
    if !present(store)? {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(store.root().join(DIRECTORY)).map_err(|_| AppError::storage())? {
        if records.len() >= LIMIT {
            return Err(unverified());
        }
        let entry = entry.map_err(|_| AppError::storage())?;
        let path = entry.path();
        reject_links(&path)?;
        if !entry
            .file_type()
            .map_err(|_| AppError::storage())?
            .is_file()
        {
            return Err(unverified());
        }
        let pointer: Pointer = read_json(&path, 4096)?;
        let record: Record = refs.get("k2", &pointer.reference)?;
        validate(&record)?;
        if pointer.schema_version != 1
            || path.file_name().and_then(|n| n.to_str())
                != Some(format!("{}.json", record.key.fingerprint()?).as_str())
        {
            return Err(unverified());
        }
        records.push(record);
    }
    Ok(records)
}

#[cfg(test)]
mod tests;
fn validate(record: &Record) -> Result<()> {
    record.key.validate()?;
    let target = &record.source;
    if record.schema_version != 1
        || target.installation_digest != record.key.archive()
        || target.input_hash.len() != 64
        || !target.input_hash.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(unverified());
    }
    for id in [&target.instance_id, &target.run_id, &target.installation_id] {
        crate::core::domain::request_key(id).map_err(|_| unverified())?;
    }
    Ok(())
}
pub(super) fn supported(store: &Store, refs: &References, key: &Key) -> Result<bool> {
    key.validate()?;
    Ok(records(store, refs)?
        .iter()
        .any(|record| record.key == *key))
}
pub(super) fn record(
    tx: &Transaction<'_>,
    refs: &References,
    key: Key,
    source: Target,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    let record = Record {
        schema_version: 1,
        key,
        source,
    };
    validate(&record)?;
    guard()?;
    tx.verify_lock()?;
    let existing = records(tx.store(), refs)?;
    if existing.iter().any(|r| r.key == record.key) {
        tx.verify_lock()?;
        return guard();
    }
    if existing.len() >= LIMIT {
        return Err(unverified());
    }
    let directory = tx.store().root().join(DIRECTORY);
    guard()?;
    tx.verify_lock()?;
    private_dir(&directory)?;
    let path = directory.join(format!("{}.json", record.key.fingerprint()?));
    reject_links(&path)?;
    guard()?;
    tx.verify_lock()?;
    let reference = refs.issue("k2", &record)?;
    guard()?;
    tx.verify_lock()?;
    atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )?;
    tx.verify_lock()?;
    guard()
}

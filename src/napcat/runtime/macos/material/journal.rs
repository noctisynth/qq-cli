//! Signed r2 persistence; no credential or configuration effects.
use super::{Pointer, Record, invalid};
use crate::{
    core::error::Result,
    napcat::runtime::{runtime_store, runtime_store::Plan},
    storage::{
        config::read_json,
        references::References,
        state::{Store, Transaction, atomic_json, private_dir, reject_links},
    },
};
use std::path::{Path, PathBuf};

pub(super) fn path(store: &Store, plan: &Plan) -> Result<PathBuf> {
    reject_links(store.root())?;
    if store.root().canonicalize().map_err(|_| invalid())? != store.root() {
        return Err(invalid());
    }
    Ok(store.root().join("macos-materials").join(format!(
        "{}.json",
        runtime_store::identity(&plan.instance_id, &plan.run_id)?
    )))
}
pub(super) fn load(path: &Path, refs: &References) -> Result<Option<Record>> {
    reject_links(path)?;
    let pointer: Pointer = match read_json(path, 4096) {
        Ok(v) => v,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.schema_version != 1 {
        return Err(invalid());
    }
    let record: Record = refs.get("r2", &pointer.reference)?;
    crate::core::domain::request_key(&record.request).map_err(|_| invalid())?;
    if record.schema_version != 1 || record.secret_digest.len() != 64 {
        return Err(invalid());
    }
    Ok(Some(record))
}
pub(super) fn save(
    tx: &Transaction<'_>,
    lifecycle: &crate::storage::state::durable::Lock,
    refs: &References,
    path: &Path,
    record: &Record,
) -> Result<()> {
    tx.verify_lock()?;
    lifecycle.verify()?;
    private_dir(path.parent().ok_or_else(invalid)?)?;
    let reference = refs.issue("r2", record)?;
    tx.verify_lock()?;
    lifecycle.verify()?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )?;
    tx.verify_lock()?;
    lifecycle.verify()
}

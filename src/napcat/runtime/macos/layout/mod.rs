//! Authenticated fixed directory identities, not credentials, ready-state or a
//! permit to execute/delete arbitrary contents subsequently placed in the tree.
use crate::{
    core::error::{AppError, Result},
    storage::{
        config::read_json,
        references::References,
        state::{
            Installation, Instance, Store, Transaction, atomic_json, private_dir, reject_links,
        },
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
mod fs;
mod profile;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    instance: String,
    run: String,
    installation: String,
    digest: String,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "snapshot",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Outcome {
    Pending,
    Unknown,
    Complete(fs::Snapshot),
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u8,
    layout_version: u8,
    request: String,
    target: Target,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    profile: Option<String>,
    outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema_version: u8,
    reference: String,
}

pub enum Preparation {
    Ready(Layout),
    Unknown,
}
pub struct Layout {
    root: PathBuf,
    snapshot: fs::Snapshot,
    fingerprint: String,
    profile: profile::Profile,
}
impl Layout {
    pub fn directory(&self) -> &Path {
        &self.root
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn profile_home(&self) -> &Path {
        self.profile.home()
    }
    /// Checks original fixed directory identities and private mode/UID/ACL.
    /// It intentionally does not authenticate arbitrary later child contents.
    pub fn verify(&self, deadline: Instant) -> Result<()> {
        self.snapshot.verify(&self.root, false, deadline)?;
        self.profile.verify(&self.root, deadline)
    }
}
fn layout(
    root: PathBuf,
    record: &Record,
    refs: &References,
    deadline: Instant,
) -> Result<Preparation> {
    let Outcome::Complete(snapshot) = &record.outcome else {
        return Ok(Preparation::Unknown);
    };
    snapshot.verify(&root, false, deadline)?;
    let profile = profile::restore(&root, record, refs, deadline)?;
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-layout-v1\0");
    hash.update(serde_json::to_vec(record).map_err(|_| invalid())?);
    Ok(Preparation::Ready(Layout {
        root,
        snapshot: snapshot.clone(),
        fingerprint: format!("{:x}", hash.finalize()),
        profile,
    }))
}
fn scope(tx: &Transaction<'_>, id: &str) -> Result<(Target, Instance, Installation)> {
    let mut instances = tx.registry.instances.iter().filter(|v| v.id == id);
    let instance = instances.next().ok_or_else(invalid)?;
    if instances.next().is_some() {
        return Err(invalid());
    }
    let mut installations = tx
        .registry
        .installations
        .iter()
        .filter(|v| v.id == instance.installation_id);
    let installation = installations.next().ok_or_else(invalid)?;
    if installations.next().is_some() {
        return Err(invalid());
    }
    for value in [&instance.id, &instance.run_id, &installation.id] {
        crate::core::domain::request_key(value).map_err(|_| invalid())?;
    }
    if installation.digest.len() != 64
        || !installation
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    Ok((
        Target {
            instance: instance.id.clone(),
            run: instance.run_id.clone(),
            installation: installation.id.clone(),
            digest: installation.digest.clone(),
        },
        instance.clone(),
        installation.clone(),
    ))
}
fn paths(store: &Store, target: &Target) -> Result<(PathBuf, PathBuf)> {
    reject_links(store.root())?;
    let base = store.root().canonicalize().map_err(|_| invalid())?;
    if base != store.root() {
        return Err(invalid());
    }
    let id = crate::napcat::runtime::runtime_store::identity(&target.instance, &target.run)?;
    Ok((
        base.join("macos-layouts").join(format!("{id}.json")),
        base.join("runtimes").join(id),
    ))
}
fn load(path: &Path, refs: &References) -> Result<Option<Record>> {
    Ok(load_bound(path, refs)?.map(|(record, _)| record))
}
fn load_bound(path: &Path, refs: &References) -> Result<Option<(Record, String)>> {
    reject_links(path)?;
    let pointer: Pointer = match read_json(path, 4096) {
        Ok(v) => v,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.schema_version != 1 {
        return Err(invalid());
    }
    let record = authenticate(refs, &pointer.reference)?;
    Ok(Some((record, pointer.reference)))
}
fn authenticate(refs: &References, reference: &str) -> Result<Record> {
    let record: Record = refs.get("l2", reference)?;
    if !matches!(
        (record.schema_version, record.layout_version),
        (1, 2) | (2, 3)
    ) || (record.layout_version == 2 && record.profile.is_some())
    {
        return Err(invalid());
    }
    crate::core::domain::request_key(&record.request).map_err(|_| invalid())?;
    for key in [
        &record.target.instance,
        &record.target.run,
        &record.target.installation,
    ] {
        crate::core::domain::request_key(key).map_err(|_| invalid())?;
    }
    if record.target.digest.len() != 64
        || !record
            .target
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    Ok(record)
}
fn save(
    tx: &Transaction<'_>,
    lock: &crate::storage::state::durable::Lock,
    refs: &References,
    path: &Path,
    record: &Record,
) -> Result<()> {
    tx.verify_lock()?;
    lock.verify()?;
    private_dir(path.parent().ok_or_else(invalid)?)?;
    let reference = refs.issue("l2", record)?;
    tx.verify_lock()?;
    lock.verify()?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )?;
    tx.verify_lock()?;
    lock.verify()
}

/// No repair, adoption or promotion of instance state during recovery.
pub fn recover(
    store: &Store,
    refs: &References,
    instance_id: &str,
    deadline: Instant,
) -> Result<Option<Preparation>> {
    crate::core::deadline::check(deadline)?;
    let tx = store.transaction(deadline)?;
    recover_in(&tx, refs, instance_id, deadline)
}
pub(super) fn recover_in(
    tx: &Transaction<'_>,
    refs: &References,
    instance_id: &str,
    deadline: Instant,
) -> Result<Option<Preparation>> {
    crate::core::deadline::check(deadline)?;
    let (target, _, _) = scope(tx, instance_id)?;
    let (path, root) = paths(tx.store(), &target)?;
    let Some(record) = load(&path, refs)? else {
        return Ok(None);
    };
    if record.target != target {
        return Err(invalid());
    }
    let result = layout(root, &record, refs, deadline)?;
    tx.verify_lock()?;
    Ok(Some(result))
}

pub fn prepare(
    store: &Store,
    refs: &References,
    instance_id: &str,
    request_id: &str,
    deadline: Instant,
) -> Result<Preparation> {
    prepare_with(store, refs, instance_id, request_id, deadline, fs::create)
}
fn prepare_with(
    store: &Store,
    refs: &References,
    instance_id: &str,
    request_id: &str,
    deadline: Instant,
    operation: impl FnOnce(&Path, u64, Instant, &dyn Fn() -> Result<()>) -> Result<fs::Snapshot>,
) -> Result<Preparation> {
    crate::core::deadline::check(deadline)?;
    crate::core::domain::request_key(request_id).map_err(|_| invalid())?;
    let lifecycle = crate::napcat::runtime::lifecycle_lock(store, deadline)?;
    let tx = store.transaction(deadline)?;
    let snapshot = scope(&tx, instance_id)?;
    let state_lock = tx.lock_identity()?;
    let (target, instance, _) = &snapshot;
    let (path, root) = paths(store, target)?;
    if let Some(record) = load(&path, refs)? {
        if record.target != *target || record.request != request_id {
            return Err(invalid());
        }
        let result = layout(root, &record, refs, deadline)?;
        tx.verify_lock()?;
        lifecycle.verify()?;
        return Ok(result);
    }
    if !["preparing", "prepared"].contains(&instance.state.as_str())
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
    {
        return Err(invalid());
    }
    reject_links(&root)?;
    if root.try_exists().map_err(|_| invalid())? {
        return Err(invalid());
    }
    let mut record = Record {
        schema_version: 2,
        layout_version: 3,
        request: request_id.into(),
        target: target.clone(),
        profile: profile::inherit(&tx, refs, instance, deadline)?,
        outcome: Outcome::Pending,
    };
    save(&tx, &lifecycle, refs, &path, &record)?;
    drop(tx);
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()?;
        if record.profile.is_some() {
            profile::restore(&root, &record, refs, deadline)?;
        }
        let tx = store.transaction(deadline)?;
        if tx.lock_identity()? != state_lock
            || scope(&tx, instance_id)? != snapshot
            || load(&path, refs)?.as_ref() != Some(&record)
        {
            return Err(invalid());
        }
        tx.verify_lock()?;
        lifecycle.verify()
    };
    let result: Result<fs::Snapshot> = (|| {
        guard()?;
        private_dir(root.parent().ok_or_else(invalid)?)?;
        guard()?;
        let result = operation(&root, state_lock.0, deadline, &guard)?;
        result.verify(&root, true, deadline)?;
        guard()?;
        Ok(result)
    })();
    guard().map_err(|_| unknown())?;
    let tx = store.transaction(deadline).map_err(|_| unknown())?;
    if tx.lock_identity()? != state_lock
        || scope(&tx, instance_id)? != snapshot
        || load(&path, refs)?.as_ref() != Some(&record)
    {
        return Err(unknown());
    }
    record.outcome = match result {
        Ok(snapshot) => Outcome::Complete(snapshot),
        Err(_) => Outcome::Unknown,
    };
    save(&tx, &lifecycle, refs, &path, &record).map_err(|_| unknown())?;
    layout(root, &record, refs, deadline)
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_LAYOUT_CONFLICT",
        "Runtime directory identities or preparation scope changed; unknown material was preserved.",
        4,
    )
}
fn unknown() -> AppError {
    AppError::new(
        "MACOS_LAYOUT_OUTCOME_UNKNOWN",
        "Runtime layout preparation is unresolved; preserve its files and do not replay it.",
        3,
    )
}

#[cfg(test)]
mod tests;

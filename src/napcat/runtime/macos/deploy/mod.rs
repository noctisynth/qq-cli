//! At-most-once instance app deployment. Authenticated observations never restore
//! mutation authority or grant process ownership/startup permission.
use crate::core::error::{AppError, Result};
use crate::napcat::install::macos::bundle::{Artifact, Inventory};
use crate::storage::{
    config::read_json,
    references::References,
    state::{Installation, Instance, Store, Transaction, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

pub use crate::napcat::runtime::start_intent::Target;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Complete,
    Unknown,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "artifact",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Outcome {
    Pending,
    Unknown,
    Complete(Artifact),
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    target: Target,
    outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

fn validate(target: &Target) -> Result<()> {
    for id in [&target.instance_id, &target.run_id, &target.installation_id] {
        crate::core::domain::request_key(id).map_err(|_| invalid())?;
    }
    for hash in [&target.input_hash, &target.installation_digest] {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
    }
    Ok(())
}
fn paths(store: &Store, target: &Target) -> Result<(PathBuf, PathBuf)> {
    validate(target)?;
    let base = store.root().canonicalize().map_err(|_| invalid())?;
    reject_links(store.root())?;
    let id = crate::napcat::runtime::runtime_store::identity(&target.instance_id, &target.run_id)?;
    Ok((
        base.join("macos-deployments").join(format!("{id}.json")),
        base.join("runtimes").join(id).join("QQ.app"),
    ))
}
fn binding(transaction: &Transaction<'_>, target: &Target) -> Result<(Instance, Installation)> {
    let mut instances = transaction
        .registry
        .instances
        .iter()
        .filter(|i| i.id == target.instance_id);
    let instance = instances.next().ok_or_else(invalid)?;
    if instances.next().is_some()
        || instance.run_id != target.run_id
        || instance.installation_id != target.installation_id
    {
        return Err(invalid());
    }
    let mut installations = transaction
        .registry
        .installations
        .iter()
        .filter(|i| i.id == target.installation_id);
    let installation = installations.next().ok_or_else(invalid)?;
    if installations.next().is_some() || installation.digest != target.installation_digest {
        return Err(invalid());
    }
    Ok((instance.clone(), installation.clone()))
}
fn load(path: &Path, references: &References) -> Result<Option<Record>> {
    reject_links(path)?;
    let pointer: Pointer = match read_json(path, 4096) {
        Ok(v) => v,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.schema_version != 1 {
        return Err(invalid());
    }
    let record: Record = references.get("d2", &pointer.reference)?;
    if record.schema_version != 1 {
        return Err(invalid());
    }
    validate(&record.target)?;
    crate::core::domain::request_key(&record.request_id).map_err(|_| invalid())?;
    Ok(Some(record))
}
fn save(
    transaction: &Transaction<'_>,
    lifecycle: &crate::storage::state::durable::Lock,
    references: &References,
    path: &Path,
    record: &Record,
) -> Result<()> {
    transaction.verify_lock()?;
    lifecycle.verify()?;
    private_dir(path.parent().ok_or_else(invalid)?)?;
    let reference = references.issue("d2", record)?;
    transaction.verify_lock()?;
    lifecycle.verify()?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )?;
    transaction.verify_lock()?;
    lifecycle.verify()
}

/// Read the original scope's authenticated state. Pending means unknown, never
/// resumable. Completed artifacts are re-read; no owner capability is returned.
pub fn recover(
    store: &Store,
    references: &References,
    target: &Target,
    deadline: Instant,
) -> Result<Option<Status>> {
    crate::core::deadline::check(deadline)?;
    let (path, root) = paths(store, target)?;
    let transaction = store.transaction(deadline)?;
    binding(&transaction, target)?;
    let Some(record) = load(&path, references)? else {
        return Ok(None);
    };
    if &record.target != target {
        return Err(invalid());
    }
    let status = match record.outcome {
        Outcome::Complete(artifact) => {
            artifact.verify(&root, deadline)?;
            Status::Complete
        }
        Outcome::Pending | Outcome::Unknown => Status::Unknown,
    };
    transaction.verify_lock()?;
    crate::core::deadline::check(deadline)?;
    Ok(Some(status))
}

/// Presence check only: no credentials, mutation or deployment authority.
pub fn has_record(store: &Store, instance: &Instance) -> Result<bool> {
    let id = crate::napcat::runtime::runtime_store::identity(&instance.id, &instance.run_id)?;
    let path = store
        .root()
        .join("macos-deployments")
        .join(format!("{id}.json"));
    reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}

/// Recover the original completed deployment target without requiring a mounted
/// Source again. This observes an authenticated artifact, never restores a writer.
pub fn completed_target(
    store: &Store,
    references: &References,
    instance: &Instance,
    deadline: Instant,
) -> Result<Option<Target>> {
    Ok(completed_inventory(store, references, instance, deadline)?.map(|v| v.target))
}
pub(super) struct Completed {
    pub target: Target,
    pub inventory: Inventory,
    record: Record,
}
impl Completed {
    pub fn receipt(&self) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&self.record).map_err(|_| invalid())?)
        ))
    }
    pub fn verify_in(
        &self,
        tx: &Transaction<'_>,
        refs: &References,
        deadline: Instant,
    ) -> Result<()> {
        let (path, _) = paths(tx.store(), &self.target)?;
        binding(tx, &self.target)?;
        if load(&path, refs)?.as_ref() != Some(&self.record) {
            return Err(invalid());
        }
        self.inventory.verify_identity(deadline)?;
        tx.verify_lock()
    }
}
pub(super) fn completed_inventory(
    store: &Store,
    references: &References,
    instance: &Instance,
    deadline: Instant,
) -> Result<Option<Completed>> {
    let id = crate::napcat::runtime::runtime_store::identity(&instance.id, &instance.run_id)?;
    let path = store
        .root()
        .join("macos-deployments")
        .join(format!("{id}.json"));
    let tx = store.transaction(deadline)?;
    let Some(record) = load(&path, references)? else {
        return Ok(None);
    };
    let (current, _) = binding(&tx, &record.target)?;
    if &current != instance {
        return Err(invalid());
    }
    let Outcome::Complete(artifact) = &record.outcome else {
        return Err(unknown());
    };
    let (_, root) = paths(store, &record.target)?;
    let inventory = artifact.inspect(&root, deadline)?;
    tx.verify_lock()?;
    Ok(Some(Completed {
        target: record.target.clone(),
        inventory,
        record,
    }))
}

/// Final readonly artifact check under the existing dispatch transaction.
pub(super) fn verify_in(
    tx: &Transaction<'_>,
    references: &References,
    target: &Target,
    deadline: Instant,
) -> Result<()> {
    let (path, root) = paths(tx.store(), target)?;
    binding(tx, target)?;
    let record = load(&path, references)?.ok_or_else(invalid)?;
    if &record.target != target {
        return Err(invalid());
    }
    let Outcome::Complete(artifact) = record.outcome else {
        return Err(unknown());
    };
    artifact.verify(&root, deadline)?;
    tx.verify_lock()
}

/// Caller must authenticate the complete fixed deployment inputs represented
/// by input_hash. Only a fresh durable intent invokes the supplied operation.
/// The operation must use the supplied guard at its real effect boundaries.
pub fn deploy(
    store: &Store,
    references: &References,
    request_id: &str,
    target: &Target,
    deadline: Instant,
    operation: impl FnOnce(&Path, &dyn Fn() -> Result<()>) -> Result<Inventory>,
) -> Result<Status> {
    crate::core::deadline::check(deadline)?;
    crate::core::domain::request_key(request_id).map_err(|_| invalid())?;
    validate(target)?;
    let lifecycle = crate::napcat::runtime::lifecycle_lock(store, deadline)?;
    let (path, root) = paths(store, target)?;
    let transaction = store.transaction(deadline)?;
    let snapshot = binding(&transaction, target)?;
    let state_lock = transaction.lock_identity()?;
    if let Some(mut record) = load(&path, references)? {
        if record.target != *target || record.request_id != request_id {
            return Err(invalid());
        }
        return match &record.outcome {
            Outcome::Complete(artifact) => {
                artifact.verify(&root, deadline)?;
                transaction.verify_lock()?;
                lifecycle.verify()?;
                Ok(Status::Complete)
            }
            _ => {
                record.outcome = Outcome::Unknown;
                save(&transaction, &lifecycle, references, &path, &record)?;
                Ok(Status::Unknown)
            }
        };
    }
    let (instance, _) = &snapshot;
    if instance.state != "prepared"
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
    let pending = Record {
        schema_version: 1,
        request_id: request_id.into(),
        target: target.clone(),
        outcome: Outcome::Pending,
    };
    save(&transaction, &lifecycle, references, &path, &pending)?;
    drop(transaction);
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()?;
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock
            || binding(&transaction, target)? != snapshot
            || load(&path, references)?.as_ref() != Some(&pending)
        {
            return Err(invalid());
        }
        transaction.verify_lock()?;
        lifecycle.verify()
    };
    guard()?;
    let result = operation(&root, &guard)
        .and_then(|inventory| inventory.finish_deployment(&root, deadline, &guard));
    // After an effect, failure/lost authority never permits another dispatch.
    // If cancellation prevents publication the durable Pending remains unknown.
    guard().map_err(|_| unknown())?;
    let transaction = store.transaction(deadline).map_err(|_| unknown())?;
    if transaction.lock_identity()? != state_lock
        || binding(&transaction, target)? != snapshot
        || load(&path, references)?.as_ref() != Some(&pending)
    {
        return Err(unknown());
    }
    let mut record = pending;
    record.outcome = match result {
        Ok(artifact) => Outcome::Complete(artifact),
        Err(_) => Outcome::Unknown,
    };
    save(&transaction, &lifecycle, references, &path, &record).map_err(|_| unknown())?;
    crate::core::deadline::check(deadline).map_err(|_| unknown())?;
    Ok(if matches!(record.outcome, Outcome::Complete(_)) {
        Status::Complete
    } else {
        Status::Unknown
    })
}
fn invalid() -> AppError {
    AppError::new(
        "DEPLOYMENT_CONFLICT",
        "Deployment state, scope or inputs changed; no repeat deployment was authorized.",
        4,
    )
}
fn unknown() -> AppError {
    AppError::new(
        "DEPLOYMENT_OUTCOME_UNKNOWN",
        "Deployment outcome is unknown; retain the partial copy and do not repeat it.",
        3,
    )
}

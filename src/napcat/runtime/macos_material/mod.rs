//! One-shot credential/config preparation; neither deployment nor a start permit.
use super::{
    macos_config,
    macos_layout::{self, Layout},
    runtime_config::Bundle,
    runtime_store::{self, Plan},
};
use crate::{
    core::error::{AppError, Result},
    storage::{
        config::read_json,
        credentials::Credentials,
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
mod secrets;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Target {
    plan: Plan,
    instance: Instance,
    installation: Installation,
    layout: String,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "state",
    content = "files",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum Outcome {
    Pending,
    Unknown,
    Complete(macos_config::Published),
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u8,
    request: String,
    target: Target,
    secret_digest: String,
    outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub enum Preparation {
    Ready(Prepared),
    Unknown,
}
/// Secret-bearing, deliberately neither Debug nor Serialize.
pub struct Prepared {
    bundle: Bundle,
    directory: PathBuf,
}
impl Prepared {
    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_MATERIAL_CONFLICT",
        "Runtime preparation authority or private material does not match; no replacement was authorized.",
        4,
    )
}
fn unknown() -> AppError {
    AppError::new(
        "MACOS_MATERIAL_OUTCOME_UNKNOWN",
        "Runtime preparation is unresolved; preserve its private material and do not replay it.",
        3,
    )
}
fn scope(tx: &Transaction<'_>, plan: &Plan) -> Result<(Instance, Installation)> {
    plan.ports.validate()?;
    let release = crate::napcat::install::catalog::lookup("4.18.41")?;
    if plan.version != release.version {
        return Err(invalid());
    }
    runtime_store::identity(&plan.instance_id, &plan.run_id)?;
    let mut found = tx
        .registry
        .instances
        .iter()
        .filter(|v| v.id == plan.instance_id);
    let instance = found.next().ok_or_else(invalid)?;
    if found.next().is_some()
        || instance.run_id != plan.run_id
        || !["preparing", "prepared"].contains(&instance.state.as_str())
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
        || instance.endpoint != format!("http://127.0.0.1:{}/", plan.ports.http)
        || instance.ws_endpoint != Some(format!("ws://127.0.0.1:{}/event", plan.ports.events))
        || instance.credential_key
            != runtime_store::api_credential_key(&plan.instance_id, &plan.run_id)?
    {
        return Err(invalid());
    }
    let mut found = tx
        .registry
        .installations
        .iter()
        .filter(|v| v.id == instance.installation_id);
    let installation = found.next().ok_or_else(invalid)?;
    if found.next().is_some()
        || installation.version != release.version
        || installation.digest != release.archive.sha256
    {
        return Err(invalid());
    }
    Ok((instance.clone(), installation.clone()))
}
fn path(store: &Store, plan: &Plan) -> Result<PathBuf> {
    reject_links(store.root())?;
    if store.root().canonicalize().map_err(|_| invalid())? != store.root() {
        return Err(invalid());
    }
    Ok(store.root().join("macos-materials").join(format!(
        "{}.json",
        runtime_store::identity(&plan.instance_id, &plan.run_id)?
    )))
}
fn load(path: &Path, refs: &References) -> Result<Option<Record>> {
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
fn save(
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
fn layout(store: &Store, refs: &References, plan: &Plan, deadline: Instant) -> Result<Layout> {
    match macos_layout::recover(store, refs, &plan.instance_id, deadline)? {
        Some(macos_layout::Preparation::Ready(v)) => Ok(v),
        _ => Err(invalid()),
    }
}
fn matches(
    record: &Record,
    plan: &Plan,
    current: &(Instance, Installation),
    layout: &Layout,
) -> bool {
    let mut expected = record.target.instance.clone();
    // Promotion is the only supported change after a completed preparation.
    if matches!(record.outcome, Outcome::Complete(_)) && current.0.state == "prepared" {
        expected.state = "prepared".into();
    }
    record.target.plan == *plan
        && expected == current.0
        && record.target.installation == current.1
        && record.target.layout == layout.fingerprint()
}
fn restored(
    record: &Record,
    layout: &Layout,
    credentials: &impl Credentials,
    deadline: Instant,
) -> Result<Preparation> {
    let Outcome::Complete(files) = &record.outcome else {
        return Ok(Preparation::Unknown);
    };
    let bundle = secrets::recover(credentials, record, deadline)?;
    files.verify(layout, &bundle, deadline)?;
    Ok(Preparation::Ready(Prepared {
        bundle,
        directory: layout.directory().into(),
    }))
}
/// Read-only prelaunch recovery. Never recreates missing credentials or promotes state.
pub fn recover(
    store: &Store,
    refs: &References,
    credentials: &impl Credentials,
    plan: &Plan,
    deadline: Instant,
) -> Result<Option<Preparation>> {
    let layout = layout(store, refs, plan, deadline)?;
    let tx = store.transaction(deadline)?;
    let current = scope(&tx, plan)?;
    let state_lock = tx.lock_identity()?;
    let Some(record) = load(&path(store, plan)?, refs)? else {
        return Ok(None);
    };
    if !matches(&record, plan, &current, &layout) {
        return Err(invalid());
    }
    tx.verify_lock()?;
    drop(tx);
    let result = restored(&record, &layout, credentials, deadline)?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != state_lock
        || scope(&tx, plan)? != current
        || load(&path(store, plan)?, refs)?.as_ref() != Some(&record)
    {
        return Err(invalid());
    }
    tx.verify_lock()?;
    Ok(Some(result))
}
pub fn prepare(
    store: &Store,
    refs: &References,
    credentials: &impl Credentials,
    plan: &Plan,
    request: &str,
    deadline: Instant,
) -> Result<Preparation> {
    crate::core::deadline::check(deadline)?;
    crate::core::domain::request_key(request).map_err(|_| invalid())?;
    let lifecycle = super::lifecycle_lock(store, deadline)?;
    let layout = layout(store, refs, plan, deadline)?;
    let tx = store.transaction(deadline)?;
    let original = scope(&tx, plan)?;
    let state_lock = tx.lock_identity()?;
    let path = path(store, plan)?;
    if let Some(record) = load(&path, refs)? {
        if record.request != request || !matches(&record, plan, &original, &layout) {
            return Err(invalid());
        }
        tx.verify_lock()?;
        drop(tx);
        let result = restored(&record, &layout, credentials, deadline)?;
        let tx = store.transaction(deadline)?;
        if tx.lock_identity()? != state_lock
            || scope(&tx, plan)? != original
            || load(&path, refs)?.as_ref() != Some(&record)
        {
            return Err(invalid());
        }
        tx.verify_lock()?;
        lifecycle.verify()?;
        if matches!(result, Preparation::Ready(_)) {
            promote(tx, &lifecycle, plan)?;
        }
        return Ok(result);
    }
    if original.0.state != "preparing" {
        return Err(invalid());
    }
    let target = Target {
        plan: plan.clone(),
        instance: original.0.clone(),
        installation: original.1.clone(),
        layout: layout.fingerprint().into(),
    };
    tx.verify_lock()?;
    drop(tx);
    macos_config::require_empty(&layout, deadline)?;
    secrets::require_absent(credentials, plan, deadline)?;
    let (bundle, raw) = secrets::generate(&target)?;
    let mut record = Record {
        schema_version: 1,
        request: request.into(),
        target,
        secret_digest: secrets::digest(&raw),
        outcome: Outcome::Pending,
    };
    lifecycle.verify()?;
    layout.verify(deadline)?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != state_lock
        || scope(&tx, plan)? != original
        || load(&path, refs)?.is_some()
    {
        return Err(invalid());
    }
    save(&tx, &lifecycle, refs, &path, &record)?;
    drop(tx);
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()?;
        layout.verify(deadline)?;
        let tx = store.transaction(deadline)?;
        if tx.lock_identity()? != state_lock
            || scope(&tx, plan)? != original
            || load(&path, refs)?.as_ref() != Some(&record)
        {
            return Err(invalid());
        }
        tx.verify_lock()?;
        lifecycle.verify()
    };
    let result: Result<macos_config::Published> = (|| {
        guard()?;
        secrets::publish(credentials, &record, &raw, &bundle, deadline, &guard)?;
        let published = macos_config::publish(&layout, &bundle, deadline, guard)?;
        guard()?;
        secrets::recover(credentials, &record, deadline)?;
        published.verify(&layout, &bundle, deadline)?;
        guard()?;
        Ok(published)
    })();
    guard().map_err(|_| unknown())?;
    let tx = store.transaction(deadline).map_err(|_| unknown())?;
    if tx.lock_identity()? != state_lock
        || scope(&tx, plan)? != original
        || load(&path, refs)?.as_ref() != Some(&record)
    {
        return Err(unknown());
    }
    record.outcome = match result {
        Ok(files) => Outcome::Complete(files),
        Err(_) => Outcome::Unknown,
    };
    save(&tx, &lifecycle, refs, &path, &record).map_err(|_| unknown())?;
    drop(tx);
    let result = restored(&record, &layout, credentials, deadline)?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != state_lock
        || scope(&tx, plan)? != original
        || load(&path, refs)?.as_ref() != Some(&record)
    {
        return Err(unknown());
    }
    if matches!(result, Preparation::Ready(_)) {
        promote(tx, &lifecycle, plan)?;
    }
    Ok(result)
}
fn promote(
    mut tx: Transaction<'_>,
    lifecycle: &crate::storage::state::durable::Lock,
    plan: &Plan,
) -> Result<()> {
    let (instance, _) = scope(&tx, plan)?;
    lifecycle.verify()?;
    if instance.state == "prepared" {
        return tx.verify_lock();
    }
    tx.registry
        .instances
        .iter_mut()
        .find(|v| v.id == plan.instance_id)
        .ok_or_else(invalid)?
        .state = "prepared".into();
    tx.commit()?;
    lifecycle.verify()
}
#[cfg(test)]
mod tests;

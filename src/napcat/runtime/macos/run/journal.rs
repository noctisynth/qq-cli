//! Authenticate original j2 records against registry and dispatch intent.
use super::{Platform, Pointer, Record, conflict};
use crate::{
    core::error::{AppError, Result},
    storage::{
        references::References,
        state::{Installation, Instance, Store, Transaction},
    },
};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};

pub(super) fn path(store: &Store, instance: &str, run: &str) -> Result<PathBuf> {
    for key in [instance, run] {
        crate::core::domain::request_key(key).map_err(|_| conflict())?;
    }
    let digest = Sha256::digest(serde_json::to_vec(&(instance, run)).map_err(|_| conflict())?);
    Ok(store
        .root()
        .join("macos-runs")
        .join(format!("{digest:x}.json")))
}
/// Presence routes recovery only. Legacy Windows j1/t1 records are never migrated or re-signed.
pub fn has_record(store: &Store, instance: &str, run: &str) -> Result<bool> {
    let path = path(store, instance, run)?;
    crate::storage::state::reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
pub(super) fn authenticated(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
    service: bool,
) -> Result<Record> {
    crate::core::deadline::check(deadline)?;
    tx.verify_lock()?;
    if tx
        .registry
        .instances
        .iter()
        .filter(|i| i.id == instance.id)
        .count()
        != 1
        || !tx.registry.instances.contains(instance)
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == installation.id)
            .count()
            != 1
        || !tx.registry.installations.contains(installation)
        || instance.installation_id != installation.id
        || !(if service {
            matches!(
                instance.state.as_str(),
                "starting" | "login_required" | "ready"
            )
        } else {
            matches!(
                instance.state.as_str(),
                "starting" | "login_required" | "ready" | "stopping" | "stopped"
            )
        })
    {
        return Err(conflict());
    }
    let path = path(tx.store(), &instance.id, &instance.run_id)?;
    crate::storage::state::reject_links(&path)?;
    let pointer: Pointer = crate::storage::config::read_json(&path, 4096)?;
    if pointer.schema_version != 1 {
        return Err(conflict());
    }
    let record: Record = refs.get("j2", &pointer.reference)?;
    if let Some(key) = &record.compatibility {
        key.validate()?;
    }
    let Platform::Macos = record.platform;
    if record.schema_version != 1
        || record.target.instance_id != instance.id
        || record.target.run_id != instance.run_id
        || record.target.installation_id != installation.id
        || record.target.installation_digest != installation.digest
        || record.endpoint != instance.endpoint
        || record.ws_endpoint != instance.ws_endpoint
    {
        return Err(conflict());
    }
    let intent = crate::napcat::runtime::start_intent::get(tx.store(), refs, &record.request_id)?;
    if (service && intent.outcome != crate::napcat::runtime::start_intent::Outcome::Dispatched)
        || intent.target != record.target
    {
        return Err(conflict());
    }
    crate::transport::client::Endpoint::parse(&record.endpoint)?;
    if let Some(endpoint) = &record.ws_endpoint {
        crate::transport::websocket::validate_endpoint(endpoint)?;
    }
    record.owner.validate_for_host()?;
    if let Some(scope) = &record.scope {
        let id = crate::napcat::runtime::runtime_store::identity(&instance.id, &instance.run_id)?;
        scope.validate(&tx.store().root().join("runtimes").join(id).join("QQ.app"))?;
        if record.owner.image() != scope.app().join("Contents/MacOS/QQ") {
            return Err(conflict());
        }
    }
    if let Some(signal) = &record.signal
        && signal.identity() != &record.owner
    {
        return Err(conflict());
    }
    tx.verify_lock()?;
    Ok(record)
}

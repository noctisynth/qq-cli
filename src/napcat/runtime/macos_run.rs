//! Read-only recovery of a signed macOS service binding. No adoption or journal creation.
use crate::core::error::{AppError, Result};
use crate::native::process::{Identity, Pinned};
use crate::storage::{
    references::References,
    state::{Installation, Instance, Store, Transaction},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum Platform {
    Macos,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    platform: Platform,
    request_id: String,
    target: super::start_intent::Target,
    owner: Identity,
    endpoint: String,
    ws_endpoint: Option<String>,
}
pub struct ServiceRun {
    owner: Pinned,
}
impl ServiceRun {
    pub fn owner(&self) -> &Pinned {
        &self.owner
    }
}
fn path(store: &Store, instance: &str, run: &str) -> Result<PathBuf> {
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
pub fn reopen_service(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
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
        || !matches!(
            instance.state.as_str(),
            "starting" | "login_required" | "ready"
        )
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
    let intent = super::start_intent::get(tx.store(), refs, &record.request_id)?;
    if intent.outcome != super::start_intent::Outcome::Dispatched || intent.target != record.target
    {
        return Err(conflict());
    }
    crate::transport::client::Endpoint::parse(&record.endpoint)?;
    if let Some(endpoint) = &record.ws_endpoint {
        crate::transport::websocket::validate_endpoint(endpoint)?;
    }
    record.owner.validate_for_host()?;
    let owner = Pinned::reopen(&record.owner, deadline)?;
    tx.verify_lock()?;
    crate::core::deadline::check(deadline)?;
    Ok(ServiceRun { owner })
}
fn conflict() -> AppError {
    AppError::new(
        "MACOS_RUN_CONFLICT",
        "The original macOS run, dispatch or service binding does not match; no endpoint was probed.",
        4,
    )
}

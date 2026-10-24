//! A new run of an existing stopped instance; never starts a process or copies old secrets.
mod journal;
mod version;
pub use version::maintenance_operation;
pub(crate) use version::references_installation;
pub use version::rollback_installation;
#[cfg(test)]
mod tests;
#[cfg(windows)]
use crate::napcat::runtime::{instance_creation::Evidence, startup::Inputs};
use crate::{
    core::error::{AppError, Result},
    napcat::runtime::{
        instance_creation::{Creation, transition},
        ports::Lease,
        runtime_store::Plan,
    },
    storage::{
        references::References,
        state::{Installation, Instance, Store, Transaction},
    },
};
use journal::{Record, key, source_key};
use std::time::Instant;

pub struct NextRun {
    pub creation: Creation,
    pub request_id: String,
}
#[cfg(windows)]
impl Inputs {
    /// Caller keeps the lifecycle lock through preparation and eventual dispatch.
    pub fn reserve_next_run<P: crate::napcat::doctor::Probe>(
        &self,
        store: &Store,
        refs: &References,
        source: &Instance,
        evidence: &Evidence<'_, P>,
        deadline: Instant,
    ) -> Result<NextRun> {
        let mut tx = store.transaction(deadline)?;
        let installation = self.with_revalidated(
            &tx,
            evidence.release,
            evidence.witnesses,
            evidence.probe,
            deadline,
            |_, fresh| Ok(fresh.installation().clone()),
        )?;
        reserve(
            &mut tx,
            refs,
            source,
            &installation,
            evidence.release.version,
            deadline,
        )
    }
}
pub(crate) fn has_origin_for_request(store: &Store, request_id: &str) -> Result<bool> {
    journal::exists(store, request_id)
}
/// Negative presence check only; a present file still requires authentication.
pub fn has_history(store: &Store, instance: &Instance) -> Result<bool> {
    crate::requests::keys::has_start(store, &key(&instance.id, &instance.run_id)?)
}
fn target(record: &Record) -> Result<Instance> {
    Ok(Instance {
        id: record.plan.instance_id.clone(),
        run_id: record.plan.run_id.clone(),
        installation_id: record.installation.id.clone(),
        endpoint: format!("http://127.0.0.1:{}/", record.plan.ports.http),
        ws_endpoint: Some(format!("ws://127.0.0.1:{}/event", record.plan.ports.events)),
        credential_key: super::runtime_store::api_credential_key(
            &record.plan.instance_id,
            &record.plan.run_id,
        )?,
        account: None,
        state: "preparing".into(),
        process_id: None,
        process_started: None,
    })
}
fn origin(store: &Store, refs: &References, current: &Instance) -> Result<Option<Record>> {
    let Some(record) = journal::load(store, refs, true, &key(&current.id, &current.run_id)?)?
    else {
        return Ok(None);
    };
    if !transition::matches(current, &target(&record)?)
        || journal::load(
            store,
            refs,
            false,
            &source_key(&record.source.id, &record.source.run_id)?,
        )?
        .as_ref()
            != Some(&record)
    {
        return Err(conflict());
    }
    Ok(Some(record))
}
// A known derived creation cannot be mistaken for an original run after its origin is lost.
fn known_origin(store: &Store, refs: &References, current: &Instance) -> Result<Option<Record>> {
    let record = origin(store, refs, current)?;
    if record.is_none()
        && super::instance_creation::checkpoint(store, refs, &key(&current.id, &current.run_id)?)?
            .is_some()
    {
        return Err(conflict());
    }
    Ok(record)
}
/// Read-only recovery of an authenticated generation; no missing record is repaired.
pub fn active(
    store: &Store,
    refs: &References,
    current: &Instance,
    deadline: Instant,
) -> Result<Option<(Plan, String)>> {
    crate::core::deadline::check(deadline)?;
    let Some(record) = known_origin(store, refs, current)? else {
        return Ok(None);
    };
    let tx = store.transaction(deadline)?;
    if tx.registry.instances.iter().find(|i| i.id == current.id) != Some(current)
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == record.installation.id)
            .count()
            != 1
        || !tx
            .registry
            .installations
            .iter()
            .any(|i| i == &record.installation)
    {
        return Err(conflict());
    }
    transition::Proposal {
        source: &record.source,
        installation: &record.installation,
        plan: &record.plan,
        key: &key(&current.id, &current.run_id)?,
    }
    .check(&tx, refs)?;
    Ok(Some((
        record.plan.clone(),
        key(&current.id, &current.run_id)?,
    )))
}
pub(crate) fn descends_from(
    store: &Store,
    refs: &References,
    current: &Instance,
    ancestor: &str,
) -> Result<bool> {
    Ok(current.run_id == ancestor
        || known_origin(store, refs, current)?.is_some_and(|record| record.root_run_id == ancestor))
}
#[cfg(windows)]
fn reserve(
    tx: &mut Transaction<'_>,
    refs: &References,
    source: &Instance,
    installation: &Installation,
    version: &str,
    deadline: Instant,
) -> Result<NextRun> {
    if source.installation_id != installation.id {
        return Err(conflict());
    }
    reserve_to(tx, refs, source, installation, version, deadline)
}
#[cfg(windows)]
fn reserve_to(
    tx: &mut Transaction<'_>,
    refs: &References,
    source: &Instance,
    installation: &Installation,
    version: &str,
    deadline: Instant,
) -> Result<NextRun> {
    reserve_guarded(
        tx,
        refs,
        source,
        installation,
        version,
        deadline,
        &|| Ok(()),
    )
}
fn reserve_guarded(
    tx: &mut Transaction<'_>,
    refs: &References,
    source: &Instance,
    installation: &Installation,
    version: &str,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<NextRun> {
    guard()?;
    tx.verify_lock()?;
    crate::core::deadline::check(deadline)?;
    let sources = tx
        .registry
        .installations
        .iter()
        .filter(|item| item.id == source.installation_id)
        .collect::<Vec<_>>();
    let [source_installation] = sources.as_slice() else {
        return Err(conflict());
    };
    let source_installation = (*source_installation).clone();
    if source.state != "stopped"
        || installation.version.trim_start_matches('v') != version.trim_start_matches('v')
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == installation.id)
            .count()
            != 1
        || !tx.registry.installations.iter().any(|i| i == installation)
    {
        return Err(conflict());
    }
    #[cfg(windows)]
    super::stop::require_completed(tx.store(), refs, source, &source_installation)?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    super::macos::stop::require_completed(tx.store(), refs, source, &source_installation)?;
    let maintenance_reference = super::version_intent::transition_reference(
        tx,
        refs,
        source,
        &source_installation,
        installation,
    )?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if source_installation.id != installation.id && maintenance_reference.is_none() {
        return Err(conflict());
    }
    let current = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == source.id)
        .cloned()
        .ok_or_else(conflict)?;
    let claim_key = source_key(&source.id, &source.run_id)?;
    let previous = journal::load(tx.store(), refs, false, &claim_key)?;
    let mut lease = None;
    let record = match previous {
        Some(record) => {
            if record.source != *source
                || record.installation != *installation
                || record
                    .source_installation
                    .as_ref()
                    .unwrap_or(&record.installation)
                    != &source_installation
                || record.plan.version != version
                || record.maintenance_reference != maintenance_reference
            {
                return Err(conflict());
            }
            record
        }
        None => {
            if current != *source {
                return Err(conflict());
            }
            let previous = known_origin(tx.store(), refs, source)?;
            let root = previous
                .as_ref()
                .map(|record| record.root_run_id.clone())
                .unwrap_or_else(|| source.run_id.clone());
            let rollback_installation = if source_installation.id != installation.id {
                Some(source_installation.clone())
            } else {
                previous.and_then(|record| record.rollback_installation)
            };
            let extended =
                source_installation.id != installation.id || rollback_installation.is_some();
            let ports = Lease::dynamic(deadline)?;
            let mut random = [0u8; 16];
            getrandom::fill(&mut random).map_err(|_| AppError::storage())?;
            let run = format!(
                "run-{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            if run == root || tx.registry.instances.iter().any(|i| i.run_id == run) {
                return Err(conflict());
            }
            let plan = Plan {
                instance_id: source.id.clone(),
                run_id: run,
                version: version.into(),
                ports: ports.ports(),
            };
            lease = Some(ports);
            Record {
                schema_version: if extended { 2 } else { 1 },
                source: source.clone(),
                installation: installation.clone(),
                source_installation: extended.then_some(source_installation),
                rollback_installation,
                maintenance_reference,
                plan,
                root_run_id: root,
            }
        }
    };
    let request_id = key(&record.plan.instance_id, &record.plan.run_id)?;
    let proposal = transition::Proposal {
        source,
        installation,
        plan: &record.plan,
        key: &request_id,
    };
    proposal.check(tx, refs)?;
    let existing_origin = journal::load(tx.store(), refs, true, &request_id)?;
    if existing_origin
        .as_ref()
        .is_some_and(|prior| prior != &record)
        || (current != *source && existing_origin.is_none())
    {
        return Err(conflict());
    }
    if current == *source && lease.is_none() {
        lease = Some(Lease::acquire(record.plan.ports, deadline)?);
    }
    crate::core::deadline::check(deadline)?;
    guard()?;
    tx.verify_lock()?;
    if journal::load(tx.store(), refs, false, &claim_key)?.is_none() {
        journal::save(tx.store(), refs, false, &claim_key, &record)?;
    }
    crate::core::deadline::check(deadline)?;
    guard()?;
    tx.verify_lock()?;
    if existing_origin.is_none() {
        journal::save(tx.store(), refs, true, &request_id, &record)?;
    }
    if record.maintenance_reference.is_some() {
        tx.registry.maintenance_intents.remove(&source.id);
    }
    guard()?;
    tx.verify_lock()?;
    let creation = proposal.register(tx, refs, lease, deadline, guard)?;
    guard()?;
    tx.verify_lock()?;
    Ok(NextRun {
        creation,
        request_id,
    })
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn reserve_macos(
    tx: &mut Transaction<'_>,
    refs: &References,
    source: &Instance,
    installation: &Installation,
    lifecycle: &crate::storage::state::durable::Lock,
    deadline: Instant,
) -> Result<NextRun> {
    if source.installation_id != installation.id {
        return Err(conflict());
    }
    reserve_macos_version(tx, refs, source, installation, lifecycle, deadline)
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn reserve_macos_version(
    tx: &mut Transaction<'_>,
    refs: &References,
    source: &Instance,
    installation: &Installation,
    lifecycle: &crate::storage::state::durable::Lock,
    deadline: Instant,
) -> Result<NextRun> {
    reserve_guarded(
        tx,
        refs,
        source,
        installation,
        &installation.version,
        deadline,
        &|| {
            crate::core::deadline::check(deadline)?;
            lifecycle.verify()
        },
    )
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn has_successor(
    store: &Store,
    refs: &References,
    source: &Instance,
    installation: &Installation,
) -> Result<bool> {
    let Some(record) = journal::load(store, refs, false, &source_key(&source.id, &source.run_id)?)?
    else {
        return Ok(false);
    };
    if record.source != *source || record.installation != *installation {
        return Err(conflict());
    }
    Ok(true)
}
fn conflict() -> AppError {
    AppError::new(
        "RUN_TRANSITION_CONFLICT",
        "The stopped run, installation or original generation records do not match; no replacement was selected.",
        4,
    )
}

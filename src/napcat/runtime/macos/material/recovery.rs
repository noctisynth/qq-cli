//! Read-only recovery and prelaunch checks of original private material.
use super::{
    Layout, Outcome, Preparation, Prepared, Record, invalid,
    journal::{load, path},
    layout, matches, scope, secrets, unknown,
};
use crate::{
    core::error::Result,
    napcat::runtime::{
        runtime_config::Bundle,
        runtime_store::{self, Plan},
    },
    storage::{
        credentials::Credentials,
        references::References,
        state::{Installation, Instance, Store, Transaction},
    },
};
use std::time::Instant;

pub(super) fn restored(
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
/// Final original-secret/configuration check under the existing s1 transaction.
pub(in crate::napcat::runtime::macos) fn verify_prelaunch_in(
    tx: &Transaction<'_>,
    refs: &References,
    credentials: &impl Credentials,
    plan: &Plan,
    expected: &Bundle,
    deadline: Instant,
) -> Result<()> {
    let record = load(&path(tx.store(), plan)?, refs)?.ok_or_else(invalid)?;
    let Outcome::Complete(files) = &record.outcome else {
        return Err(unknown());
    };
    let current = tx
        .registry
        .instances
        .iter()
        .find(|v| v.id == plan.instance_id)
        .ok_or_else(invalid)?;
    if !matches!(current.state.as_str(), "prepared" | "starting")
        || current.account.is_some()
        || current.process_id.is_some()
        || current.process_started.is_some()
    {
        return Err(invalid());
    }
    let mut original = record.target.instance.clone();
    original.state = current.state.clone();
    if original != *current
        || record.target.plan != *plan
        || !tx
            .registry
            .installations
            .contains(&record.target.installation)
    {
        return Err(invalid());
    }
    let Some(layout::Preparation::Ready(layout)) =
        layout::recover_in(tx, refs, &plan.instance_id, deadline)?
    else {
        return Err(invalid());
    };
    if layout.fingerprint() != record.target.layout {
        return Err(invalid());
    }
    let restored = secrets::recover(credentials, &record, deadline)?;
    if expected.api_token() != restored.api_token()
        || expected.control_token() != restored.control_token()
        || expected.jwt_secret() != restored.jwt_secret()
    {
        return Err(invalid());
    }
    files.verify(&layout, &restored, deadline)?;
    tx.verify_lock()
}

/// Recover original management secrets after launch without treating mutable
/// NapCat configuration files as the prelaunch template. Never creates material.
pub fn recover_running(
    store: &Store,
    refs: &References,
    credentials: &impl Credentials,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<Prepared> {
    if !matches!(
        instance.state.as_str(),
        "starting" | "login_required" | "ready"
    ) {
        return Err(invalid());
    }
    let id = runtime_store::identity(&instance.id, &instance.run_id)?;
    let path = store
        .root()
        .join("macos-materials")
        .join(format!("{id}.json"));
    let tx = store.transaction(deadline)?;
    let state_lock = tx.lock_identity()?;
    if !tx.registry.instances.contains(instance)
        || !tx.registry.installations.contains(installation)
    {
        return Err(invalid());
    }
    let record = load(&path, refs)?.ok_or_else(invalid)?;
    let Outcome::Complete(_) = record.outcome else {
        return Err(unknown());
    };
    let mut expected = record.target.instance.clone();
    expected.state = instance.state.clone();
    expected.account = instance.account.clone();
    expected.process_id = instance.process_id;
    expected.process_started = instance.process_started.clone();
    if expected != *instance
        || record.target.installation != *installation
        || instance.installation_id != installation.id
    {
        return Err(invalid());
    }
    tx.verify_lock()?;
    drop(tx);
    let layout = layout(store, refs, &record.target.plan, deadline)?;
    if layout.fingerprint() != record.target.layout {
        return Err(invalid());
    }
    let bundle = secrets::recover(credentials, &record, deadline)?;
    layout.verify(deadline)?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != state_lock
        || !tx.registry.instances.contains(instance)
        || !tx.registry.installations.contains(installation)
        || load(&path, refs)?.as_ref() != Some(&record)
    {
        return Err(invalid());
    }
    tx.verify_lock()?;
    Ok(Prepared {
        bundle,
        directory: layout.directory().into(),
    })
}

//! Promote a fully authenticated original dispatch only after two owned online observations.
use super::*;
use crate::{
    napcat::runtime::macos::start,
    storage::{
        config::Config,
        credentials::Credentials,
        state::{Installation, Instance, Transaction},
    },
    transport::{
        client::{Backend, SessionInfo, inspect_with_version_guard},
        owned_http::OwnedOneBot,
    },
};
use std::time::Instant;

pub fn record(
    context: &start::Context<'_, impl Credentials>,
    config: &Config,
    instance: &Instance,
    deadline: Instant,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    if instance.state != "ready" {
        return Ok(());
    }
    let key = Key::current()?;
    if local::supported(context.store, context.references, &key)? {
        return Ok(());
    }
    let lifecycle = crate::napcat::runtime::lifecycle_lock(context.store, deadline)?;
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()
    };
    let tx = context.store.transaction(deadline)?;
    let original_lock = tx.lock_identity()?;
    let installed = snapshot(&tx, instance)?;
    let original = crate::napcat::runtime::macos::run::reopen_service(
        &tx,
        context.references,
        instance,
        &installed,
        deadline,
    )?;
    // Legacy/different software remains readable, but cannot witness this
    // invocation's key. Do not reconstruct it using the new binary's templates.
    if !original.verifiable_inputs() || original.compatibility() != Some(&key) {
        tx.verify_lock()?;
        return guard();
    }
    tx.verify_lock()?;
    drop(tx);
    guard()?;
    let before = start::observe(context, instance, &installed, deadline)?;
    let token = context.credentials.get(&instance.credential_key)?;
    let backend =
        OwnedOneBot::for_process(before.service.owner(), &instance.endpoint, &token, config)?;
    let online = inspect_online(&backend, &installed, deadline)?;
    require_online(&online, instance, &installed)?;
    guard()?;
    let confirmed = inspect_online(&backend, &installed, deadline)?;
    require_online(&confirmed, instance, &installed)?;
    if confirmed.account != online.account || confirmed.version != online.version {
        return Err(unverified());
    }
    let after = start::observe(context, instance, &installed, deadline)?;
    if before.target != after.target
        || before.service.owner().identity() != after.service.owner().identity()
        || key != Key::current()?
    {
        return Err(unverified());
    }
    guard()?;
    let tx = context.store.transaction(deadline)?;
    if tx.lock_identity()? != original_lock || snapshot(&tx, instance)? != installed {
        return Err(unverified());
    }
    before.service.owner().verify(deadline)?;
    after.service.owner().verify(deadline)?;
    local::record(&tx, context.references, key, after.target, &guard)
}

fn inspect_online(
    backend: &impl Backend,
    installed: &Installation,
    deadline: Instant,
) -> Result<SessionInfo> {
    inspect_with_version_guard(backend, deadline, |version| {
        if installed.version != "v4.18.41" || version.trim_start_matches('v') != "4.18.41" {
            return Err(unverified());
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;
fn snapshot(tx: &Transaction<'_>, instance: &Instance) -> Result<Installation> {
    if tx
        .registry
        .instances
        .iter()
        .filter(|i| i.id == instance.id)
        .count()
        != 1
        || !tx.registry.instances.contains(instance)
    {
        return Err(unverified());
    }
    let mut found = tx
        .registry
        .installations
        .iter()
        .filter(|i| i.id == instance.installation_id);
    let installed = found.next().ok_or_else(unverified)?;
    if found.next().is_some() {
        return Err(unverified());
    }
    Ok(installed.clone())
}
fn require_online(info: &SessionInfo, instance: &Instance, installed: &Installation) -> Result<()> {
    if !info.online
        || instance.account.as_deref() != Some(info.account.as_str())
        || info.version.trim_start_matches('v') != installed.version.trim_start_matches('v')
        || installed.version != "v4.18.41"
    {
        return Err(unverified());
    }
    Ok(())
}

//! Preserve stable instance identity. Preflight precedes stop; the original
//! stop and successor claim share one lifecycle lock, then preparation resumes
//! only the authenticated n2 request of that successor.
use super::{compatibility, start, stop};
use crate::{
    core::error::{AppError, Result},
    napcat::{
        install::{catalog, installation, macos::source::image::acquire},
        runtime::{generation, instance_creation, lifecycle_lock},
    },
    storage::{
        credentials::Credentials,
        state::{Installation, Instance, Transaction},
    },
};
use std::time::Instant;

pub fn execute(
    context: &start::Context<'_, impl Credentials>,
    selected: &Instance,
    deadline: Instant,
) -> Result<start::Started> {
    execute_with(
        context,
        selected,
        deadline,
        preflight,
        |context, current, request, deadline| start::resume(context, current, request, deadline),
    )
}

fn execute_with<C: Credentials>(
    context: &start::Context<'_, C>,
    selected: &Instance,
    deadline: Instant,
    preflight: impl FnOnce(&start::Context<'_, C>, &Instance, &Installation, Instant) -> Result<()>,
    resume: impl FnOnce(&start::Context<'_, C>, &Instance, &str, Instant) -> Result<start::Started>,
) -> Result<start::Started> {
    crate::core::deadline::check(deadline)?;
    let lifecycle = lifecycle_lock(context.store, deadline)?;
    let tx = context.store.transaction(deadline)?;
    let original_state_lock = tx.lock_identity()?;
    let (current, installed) = binding(&tx, selected)?;
    instance_creation::require_unremoved(context.store, &selected.id)?;
    if current == *selected && matches!(current.state.as_str(), "preparing" | "prepared") {
        let request =
            instance_creation::macos_request(context.store, context.references, &current)?;
        tx.verify_lock()?;
        lifecycle.verify()?;
        drop(tx);
        drop(lifecycle);
        return resume(context, &current, &request, deadline);
    }
    let mut source = selected.clone();
    source.state = "stopped".into();
    let claimed =
        generation::has_successor(context.store, context.references, &source, &installed)?;
    if !claimed && current != *selected {
        return Err(changed());
    }
    if !claimed
        && !matches!(
            current.state.as_str(),
            "starting" | "login_required" | "ready" | "stopping" | "stopped"
        )
    {
        return Err(changed());
    }
    tx.verify_lock()?;
    lifecycle.verify()?;
    drop(tx);
    if !claimed {
        preflight(context, &current, &installed, deadline)?;
        let tx = context.store.transaction(deadline)?;
        if tx.lock_identity()? != original_state_lock
            || binding(&tx, selected)? != (current.clone(), installed.clone())
        {
            return Err(changed());
        }
        tx.verify_lock()?;
        lifecycle.verify()?;
        drop(tx);
        stop::locked(
            context.store,
            context.references,
            &current,
            &lifecycle,
            deadline,
        )?;
    }
    let mut tx = context.store.transaction(deadline)?;
    if tx.lock_identity()? != original_state_lock {
        return Err(changed());
    }
    let (_, latest_installation) = binding(&tx, selected)?;
    if latest_installation != installed {
        return Err(changed());
    }
    lifecycle.verify()?;
    let next = generation::reserve_macos(
        &mut tx,
        context.references,
        &source,
        &installed,
        &lifecycle,
        deadline,
    )?;
    let request = next.request_id;
    drop(next.creation); // Release only our endpoint lease before once-only dispatch.
    let current = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == selected.id)
        .ok_or_else(changed)?
        .clone();
    tx.verify_lock()?;
    lifecycle.verify()?;
    drop(tx);
    drop(lifecycle);
    resume(context, &current, &request, deadline).map_err(|mut error| {
        if error.request_id.is_none() {
            error.request_id = Some(request);
        }
        error
    })
}

fn binding(tx: &Transaction<'_>, selected: &Instance) -> Result<(Instance, Installation)> {
    let mut instances = tx.registry.instances.iter().filter(|i| i.id == selected.id);
    let current = instances.next().ok_or_else(changed)?;
    if instances.next().is_some() {
        return Err(changed());
    }
    let mut installations = tx
        .registry
        .installations
        .iter()
        .filter(|i| i.id == selected.installation_id);
    let installed = installations.next().ok_or_else(changed)?;
    if installations.next().is_some() || current.installation_id != installed.id {
        return Err(changed());
    }
    Ok((current.clone(), installed.clone()))
}
fn preflight(
    context: &start::Context<'_, impl Credentials>,
    current: &Instance,
    installed: &Installation,
    deadline: Instant,
) -> Result<()> {
    let release = catalog::lookup("4.18.41")?;
    if installed.version != release.version || installed.digest != release.archive.sha256 {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "macOS restart requires the fixed Framework v4.18.41 installation.",
            4,
        ));
    }
    installation::verify(context.store, installed, context.references, deadline)?;
    if !compatibility::supported(
        context.store,
        context.references,
        &compatibility::Key::current()?,
    )? {
        return Err(AppError::new(
            "COMPATIBILITY_UNKNOWN",
            "Verify the original ready macOS run with napcat start before restarting this software combination.",
            4,
        ));
    }
    // Verify availability of the exact pinned image before affecting the old
    // session. Mounting/signing the successor remains in normal preparation.
    acquire::acquire(context.store, None, deadline)?;
    if matches!(
        current.state.as_str(),
        "starting" | "login_required" | "ready"
    ) {
        start::observe(context, current, installed, deadline)?;
    }
    Ok(())
}
fn changed() -> AppError {
    AppError::new(
        "RUN_TRANSITION_CONFLICT",
        "The selected macOS run or original successor changed; no replacement run was stopped.",
        4,
    )
}

#[cfg(test)]
mod tests;

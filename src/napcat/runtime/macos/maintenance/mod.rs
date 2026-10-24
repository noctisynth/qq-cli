//! Version maintenance: preflight, durable intent, original stop and successor
//! claim remain explicit under one lifecycle lock. Preparation resumes by n2.
mod preflight;
use super::{start, stop};
use crate::{
    core::error::{AppError, Result},
    napcat::runtime::{
        generation, instance_creation, lifecycle_lock,
        version_intent::{self, Operation},
    },
    storage::{
        credentials::Credentials,
        state::{Installation, Instance, Transaction, durable::Lock},
    },
};
use std::time::Instant;

struct Action<'a> {
    installation: &'a Installation,
    operation: Operation,
    trial: bool,
}

pub fn update(
    context: &start::Context<'_, impl Credentials>,
    selected: &Instance,
    target: &Installation,
    trial: bool,
    deadline: Instant,
) -> Result<start::Started> {
    execute_with(
        context,
        selected,
        Action {
            installation: target,
            operation: Operation::Update,
            trial,
        },
        deadline,
        preflight::verify,
        resume,
    )
}

pub fn rollback(
    context: &start::Context<'_, impl Credentials>,
    selected: &Instance,
    deadline: Instant,
) -> Result<start::Started> {
    let target =
        version_intent::rollback_target(context.store, context.references, selected, deadline)?;
    execute_with(
        context,
        selected,
        Action {
            installation: &target,
            operation: Operation::Rollback,
            trial: false,
        },
        deadline,
        preflight::verify,
        resume,
    )
}

fn resume(
    context: &start::Context<'_, impl Credentials>,
    current: &Instance,
    installed: &Installation,
    request: Option<&str>,
    trial: bool,
    deadline: Instant,
) -> Result<start::Started> {
    match request {
        Some(request) => start::resume(context, current, request, trial, deadline),
        None => start::selected(context, current, installed, trial, deadline),
    }
}

fn execute_with<C: Credentials>(
    context: &start::Context<'_, C>,
    selected: &Instance,
    action: Action<'_>,
    deadline: Instant,
    preflight: impl FnOnce(
        &start::Context<'_, C>,
        &Instance,
        &Installation,
        &Installation,
        bool,
        Instant,
    ) -> Result<()>,
    resume: impl FnOnce(
        &start::Context<'_, C>,
        &Instance,
        &Installation,
        Option<&str>,
        bool,
        Instant,
    ) -> Result<start::Started>,
) -> Result<start::Started> {
    crate::core::deadline::check(deadline)?;
    let lifecycle = lifecycle_lock(context.store, deadline)?;
    if action.operation == Operation::Rollback
        && version_intent::rollback_target(context.store, context.references, selected, deadline)?
            != *action.installation
    {
        return Err(version_intent::conflict());
    }
    let tx = context.store.transaction(deadline)?;
    let snapshot = Snapshot::inspect(&tx, selected, action.installation)?;
    instance_creation::require_unremoved(context.store, &selected.id)?;
    if matches!(snapshot.current.state.as_str(), "starting" | "unknown") {
        let mut error = start::unresolved();
        error.next_action = Some(Box::new(
            crate::core::contracts::action::NextAction::Status {
                instance: snapshot.current.id.clone(),
            },
        ));
        return Err(error);
    }
    if snapshot.current.run_id == selected.run_id
        && snapshot.current.installation_id == action.installation.id
    {
        let continuing = matches!(selected.state.as_str(), "preparing" | "prepared")
            && matches!(
                snapshot.current.state.as_str(),
                "preparing" | "prepared" | "ready" | "login_required"
            )
            && instance_creation::transition::matches(&snapshot.current, selected)
            && generation::maintenance_operation(
                context.store,
                context.references,
                &snapshot.current,
            )? == Some(action.operation);
        if snapshot.current != *selected && !continuing {
            return Err(version_intent::conflict());
        }
        version_intent::guard_start(
            context.store,
            context.references,
            &tx.registry,
            &snapshot.current,
        )?;
        snapshot.verify(&tx, selected, &lifecycle, deadline)?;
        drop(tx);
        preflight(
            context,
            &snapshot.current,
            &snapshot.original,
            action.installation,
            action.trial,
            deadline,
        )?;
        let tx = context.store.transaction(deadline)?;
        snapshot.verify(&tx, selected, &lifecycle, deadline)?;
        drop(tx);
        drop(lifecycle);
        return resume(
            context,
            &snapshot.current,
            action.installation,
            None,
            action.trial,
            deadline,
        );
    }
    let mut source = selected.clone();
    source.state = "stopped".into();
    let claimed = generation::has_successor(
        context.store,
        context.references,
        &source,
        action.installation,
    )?;
    if !claimed {
        let mut current = snapshot.current.clone();
        if matches!(current.state.as_str(), "stopping" | "stopped") {
            current.state = selected.state.clone();
        }
        if current != *selected {
            return Err(version_intent::conflict());
        }
    }
    snapshot.verify(&tx, selected, &lifecycle, deadline)?;
    drop(tx);
    if !claimed {
        preflight(
            context,
            &snapshot.current,
            &snapshot.original,
            action.installation,
            action.trial,
            deadline,
        )?;
    }
    // The v2 source index and active marker must both precede stop. Its guard
    // receives the existing transaction rather than recursively acquiring it.
    let intent = version_intent::begin_guarded(
        context.store,
        context.references,
        selected,
        action.installation,
        action.operation,
        deadline,
        &|tx| snapshot.verify(tx, selected, &lifecycle, deadline),
    )?;
    if intent.stopped_source() != source {
        return Err(version_intent::conflict());
    }
    if !claimed {
        stop::locked(
            context.store,
            context.references,
            &snapshot.current,
            &lifecycle,
            deadline,
        )?;
    }
    let mut tx = context.store.transaction(deadline)?;
    let after = Snapshot::inspect(&tx, selected, action.installation)?;
    if after.state_lock != snapshot.state_lock
        || after.original != snapshot.original
        || (!claimed && after.current != source)
    {
        return Err(version_intent::conflict());
    }
    lifecycle.verify()?;
    let next = generation::reserve_macos_version(
        &mut tx,
        context.references,
        &source,
        action.installation,
        &lifecycle,
        deadline,
    )?;
    let request = next.request_id;
    drop(next.creation);
    let current = crate::storage::state::select(&tx.registry, Some(&selected.id))?;
    tx.verify_lock()?;
    lifecycle.verify()?;
    drop(tx);
    drop(lifecycle);
    resume(
        context,
        &current,
        action.installation,
        Some(&request),
        action.trial,
        deadline,
    )
    .map_err(|mut error| {
        if error.request_id.is_none() {
            error.request_id = Some(request);
        }
        error
    })
}

#[derive(PartialEq, Eq)]
struct Snapshot {
    state_lock: (u64, u64, u64, u64),
    current: Instance,
    original: Installation,
    target: Installation,
}
impl Snapshot {
    fn inspect(tx: &Transaction<'_>, selected: &Instance, target: &Installation) -> Result<Self> {
        let current = crate::storage::state::select(&tx.registry, Some(&selected.id))?;
        let installation = |id: &str| -> Result<Installation> {
            let entries = tx
                .registry
                .installations
                .iter()
                .filter(|entry| entry.id == id)
                .collect::<Vec<_>>();
            match entries.as_slice() {
                [entry] => Ok((*entry).clone()),
                _ => Err(version_intent::conflict()),
            }
        };
        let original = installation(&selected.installation_id)?;
        let latest_target = installation(&target.id)?;
        if latest_target != *target {
            return Err(version_intent::conflict());
        }
        Ok(Self {
            state_lock: tx.lock_identity()?,
            current,
            original,
            target: latest_target,
        })
    }
    fn verify(
        &self,
        tx: &Transaction<'_>,
        selected: &Instance,
        lifecycle: &Lock,
        deadline: Instant,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()?;
        tx.verify_lock()?;
        if Self::inspect(tx, selected, &self.target)? != *self {
            return Err(version_intent::conflict());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;

//! Resolve the original rollback without toggling an in-progress successor.
use super::*;
use crate::napcat::runtime::{
    generation,
    version_intent::{self, Operation},
};
use crate::storage::state::{Installation, Registry};

/// Read-only resolution; returned data owns no lock. Caller rechecks after acquiring lifecycle lock.
pub fn target(
    store: &Store,
    refs: &References,
    selected: &Instance,
    deadline: Instant,
) -> Result<Installation> {
    let tx = store.transaction(deadline)?;
    let current = crate::storage::state::select(&tx.registry, Some(&selected.id))?;
    if matches!(current.state.as_str(), "starting" | "unknown") {
        return Err(unknown_instance(&current.id, None));
    }
    let target = if current.run_id != selected.run_id {
        let intent = version_intent::source_intent(store, refs, selected)?
            .ok_or_else(version_intent::conflict)?;
        if intent.operation() != Operation::Rollback {
            return Err(version_intent::conflict());
        }
        intent.target().clone()
    } else if let Some(intent) = version_intent::pending(store, refs, &tx.registry, &current)? {
        if intent.operation() != Operation::Rollback {
            return Err(version_intent::conflict());
        }
        // begin checks the source snapshot again before any stop effect.
        intent.target().clone()
    } else if matches!(selected.state.as_str(), "preparing" | "prepared")
        && matches!(
            current.state.as_str(),
            "preparing" | "prepared" | "ready" | "login_required"
        )
        && crate::napcat::runtime::instance_creation::transition::matches(&current, selected)
        && generation::maintenance_operation(store, refs, &current)? == Some(Operation::Rollback)
    {
        registered(&tx.registry, &current.installation_id)?.clone()
    } else {
        if current != *selected {
            return Err(version_intent::conflict());
        }
        generation::rollback_installation(store, refs, &current)?.ok_or_else(|| {
            AppError::new(
                "ROLLBACK_UNAVAILABLE",
                "No authenticated previous installation is recorded; no rollback was performed.",
                4,
            )
        })?
    };
    if registered(&tx.registry, &target.id)? != &target {
        return Err(version_intent::conflict());
    }
    crate::core::deadline::check(deadline)?;
    Ok(target)
}
fn registered<'a>(registry: &'a Registry, id: &str) -> Result<&'a Installation> {
    let mut found = registry.installations.iter().filter(|i| i.id == id);
    let item = found.next().ok_or_else(version_intent::conflict)?;
    if found.next().is_some() {
        return Err(version_intent::conflict());
    }
    Ok(item)
}
pub fn execute<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    let target = target(context.store, context.references, selected, deadline)?;
    if target.version.trim_start_matches('v') != context.release.version.trim_start_matches('v') {
        return Err(version_intent::conflict());
    }
    maintain_locked(context, selected, Operation::Rollback, deadline)
}

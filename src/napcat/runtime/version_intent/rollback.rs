//! Resolve a recorded rollback without toggling an in-progress successor.
use super::*;
use crate::napcat::runtime::{generation, instance_creation::transition};

/// Read only; callers revalidate the returned installation under lifecycle lock.
pub fn target(
    store: &Store,
    refs: &References,
    selected: &Instance,
    deadline: Instant,
) -> Result<Installation> {
    let tx = store.transaction(deadline)?;
    let current = crate::storage::state::select(&tx.registry, Some(&selected.id))?;
    if matches!(current.state.as_str(), "starting" | "unknown") {
        let mut error = AppError::new(
            "START_OUTCOME_UNKNOWN",
            "Startup may have been dispatched; inspect this instance and do not repeat creation with another key.",
            5,
        );
        error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
        error.next_action = Some(Box::new(
            crate::core::contracts::action::NextAction::Status {
                instance: current.id.clone(),
            },
        ));
        return Err(error);
    }
    let target = if current.run_id != selected.run_id {
        let intent = source_intent(store, refs, selected)?.ok_or_else(conflict)?;
        if intent.operation() != Operation::Rollback {
            return Err(conflict());
        }
        intent.target().clone()
    } else if let Some(intent) = pending(store, refs, &tx.registry, &current)? {
        if intent.operation() != Operation::Rollback {
            return Err(conflict());
        }
        intent.target().clone()
    } else if matches!(selected.state.as_str(), "preparing" | "prepared")
        && matches!(
            current.state.as_str(),
            "preparing" | "prepared" | "ready" | "login_required"
        )
        && transition::matches(&current, selected)
        && generation::maintenance_operation(store, refs, &current)? == Some(Operation::Rollback)
    {
        unique(&tx.registry, &current.installation_id)?.clone()
    } else {
        if current != *selected {
            return Err(conflict());
        }
        generation::rollback_installation(store, refs, &current)?.ok_or_else(|| {
            AppError::new(
                "ROLLBACK_UNAVAILABLE",
                "No authenticated previous installation is recorded; no rollback was performed.",
                4,
            )
        })?
    };
    if unique(&tx.registry, &target.id)? != &target {
        return Err(conflict());
    }
    crate::core::deadline::check(deadline)?;
    tx.verify_lock()?;
    Ok(target)
}

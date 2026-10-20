//! Registration removal preserves runtime material and write history.
mod journal;
#[cfg(test)]
mod tests;
use crate::{
    core::error::{AppError, Result},
    storage::{references::References, state::Store},
};
use journal::{Outcome, Record};
use std::time::Instant;

pub struct Removed {
    pub instance_id: String,
    pub reused: bool,
}
/// Routing only; an existing path is never removal authority.
pub fn has_record(store: &Store, instance_id: &str) -> Result<bool> {
    journal::has_record(store, instance_id)
}
pub fn execute(
    store: &Store,
    refs: &References,
    instance_id: &str,
    deadline: Instant,
) -> Result<Removed> {
    crate::core::domain::request_key(instance_id).map_err(|_| AppError::invalid())?;
    let _operation = super::operation_lock::acquire(store, deadline)?;
    let mut tx = store.transaction(deadline)?;
    let prior = journal::load(store, refs, instance_id)?;
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == instance_id)
        .cloned();
    let record = match instance {
        Some(instance) => {
            if prior.as_ref().is_some_and(|prior| {
                prior.instance != instance || prior.outcome == Outcome::Completed
            }) {
                return Err(conflict());
            }
            let installation = tx
                .registry
                .installations
                .iter()
                .find(|i| i.id == instance.installation_id)
                .ok_or_else(conflict)?;
            if tx
                .registry
                .installations
                .iter()
                .filter(|i| i.id == installation.id)
                .count()
                != 1
                || prior
                    .as_ref()
                    .is_some_and(|prior| prior.installation_digest != installation.digest)
            {
                return Err(conflict());
            }
            super::stop::require_completed(store, refs, &instance, installation)?;
            let record = Record {
                schema_version: 1,
                instance,
                installation_digest: installation.digest.clone(),
                outcome: Outcome::Pending,
            };
            journal::save(store, refs, &record)?;
            super::default_start::retire(store, refs, &record.instance)?;
            tx.registry.instances.retain(|i| i.id != instance_id);
            tx.registry.maintenance_intents.remove(instance_id);
            tx.commit().map_err(|_| unknown(instance_id))?;
            record
        }
        None => {
            let record = prior.ok_or_else(|| {
                AppError::new(
                    "INSTANCE_NOT_FOUND",
                    "No registered instance or original removal intent matches this ID.",
                    4,
                )
            })?;
            if record.outcome == Outcome::Completed {
                return Ok(Removed {
                    instance_id: instance_id.into(),
                    reused: true,
                });
            }
            // A committed removal may precede receipt persistence. Do not require the old installation.
            super::default_start::retire(store, refs, &record.instance)?;
            drop(tx);
            record
        }
    };
    journal::save(
        store,
        refs,
        &Record {
            outcome: Outcome::Completed,
            ..record
        },
    )
    .map_err(|_| unknown(instance_id))?;
    Ok(Removed {
        instance_id: instance_id.into(),
        reused: false,
    })
}
fn conflict() -> AppError {
    AppError::new(
        "REMOVE_INPUT_CHANGED",
        "The original instance, installation or removal proof changed; no replacement registration was selected.",
        4,
    )
}
fn unknown(instance_id: &str) -> AppError {
    let mut error = AppError::new(
        "REMOVE_OUTCOME_UNKNOWN",
        "Registration may have been removed; inspect this same instance ID and preserve its records.",
        5,
    );
    error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
    error.details = Some(Box::new(
        crate::core::contracts::failure::Details::Instance {
            instance_id: instance_id.into(),
        },
    ));
    error.next_action = Some(Box::new(
        crate::core::contracts::action::NextAction::Instances {},
    ));
    error
}

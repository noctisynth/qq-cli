//! Durable version intent before stop; active markers are bounded by current instances.
mod journal;
mod rollback;
mod transition;
pub use rollback::target as rollback_target;
pub(crate) use transition::{transition_reference, validate_transition};
#[cfg(test)]
mod tests;
use crate::{
    core::error::{AppError, Result},
    storage::{
        references::References,
        state::{Installation, Instance, Registry, Store, Transaction},
    },
};
pub use journal::Operation;
use journal::Record;
use std::time::Instant;

pub struct Intent {
    record: Record,
    reference: String,
}
impl Intent {
    pub fn source(&self) -> &Instance {
        &self.record.source
    }
    pub fn target(&self) -> &Installation {
        &self.record.target
    }
    pub fn operation(&self) -> Operation {
        self.record.operation
    }
    pub fn stopped_source(&self) -> Instance {
        let mut source = self.record.source.clone();
        source.state = "stopped".into();
        source
    }
}
fn checkpoint(source: &Instance, current: &Instance) -> bool {
    let mut current = current.clone();
    if matches!(current.state.as_str(), "stopping" | "stopped") {
        current.state = source.state.clone();
    }
    current == *source
}
pub fn has_pending(store: &Store, current: &Instance) -> Result<bool> {
    Ok(store.read()?.maintenance_intents.contains_key(&current.id)
        || journal::pointer(store, &current.id, &current.run_id)?.is_some())
}
pub(crate) fn pending(
    store: &Store,
    refs: &References,
    registry: &Registry,
    current: &Instance,
) -> Result<Option<Intent>> {
    if registry.instances.iter().find(|i| i.id == current.id) != Some(current) {
        return Err(conflict());
    }
    let pointer = journal::pointer(store, &current.id, &current.run_id)?;
    if registry
        .maintenance_intents
        .get(&current.id)
        .is_some_and(|reference| Some(reference) != pointer.as_ref())
    {
        return Err(conflict());
    }
    let Some(reference) = pointer else {
        return Ok(None);
    };
    let intent = journal::load(store, refs, &reference)?;
    if !checkpoint(intent.source(), current) {
        return Err(conflict());
    }
    Ok(Some(intent))
}
pub(crate) fn source_intent(
    store: &Store,
    refs: &References,
    source: &Instance,
) -> Result<Option<Intent>> {
    let Some(reference) = journal::pointer(store, &source.id, &source.run_id)? else {
        return Ok(None);
    };
    let intent = journal::load(store, refs, &reference)?;
    if !checkpoint(intent.source(), source) {
        return Err(conflict());
    }
    Ok(Some(intent))
}
/// Caller holds the lifecycle lock; publish the source claim, then the active marker, before stop.
pub fn begin(
    store: &Store,
    refs: &References,
    selected: &Instance,
    target: &Installation,
    operation: Operation,
    deadline: Instant,
) -> Result<Intent> {
    begin_guarded(store, refs, selected, target, operation, deadline, &|tx| {
        tx.verify_lock()
    })
}
pub fn begin_guarded(
    store: &Store,
    refs: &References,
    selected: &Instance,
    target: &Installation,
    operation: Operation,
    deadline: Instant,
    guard: &dyn Fn(&Transaction<'_>) -> Result<()>,
) -> Result<Intent> {
    let mut tx = store.transaction(deadline)?;
    guard(&tx)?;
    tx.verify_lock()?;
    super::instance_creation::require_unremoved(store, &selected.id)?;
    let current = crate::storage::state::select(&tx.registry, Some(&selected.id))?;
    let source_installation = unique(&tx.registry, &selected.installation_id)?.clone();
    if unique(&tx.registry, &target.id)? != target || source_installation.id == target.id {
        return Err(conflict());
    }
    let intent = match journal::pointer(store, &selected.id, &selected.run_id)? {
        Some(reference) => {
            let intent = journal::load(store, refs, &reference)?;
            if !checkpoint(intent.source(), selected)
                || intent.record.source_installation != source_installation
                || intent.target() != target
                || intent.operation() != operation
            {
                return Err(conflict());
            }
            intent
        }
        None => {
            if current != *selected || tx.registry.maintenance_intents.contains_key(&selected.id) {
                return Err(conflict());
            }
            journal::save(
                store,
                refs,
                Record {
                    schema_version: 1,
                    source: selected.clone(),
                    source_installation,
                    target: target.clone(),
                    operation,
                },
                &|| {
                    guard(&tx)?;
                    tx.verify_lock()
                },
            )?
        }
    };
    if current.run_id == selected.run_id {
        if !checkpoint(intent.source(), &current) {
            return Err(conflict());
        }
        match tx.registry.maintenance_intents.get(&selected.id) {
            Some(reference) if reference == &intent.reference => (),
            None if current == *intent.source() => {
                guard(&tx)?;
                tx.verify_lock()?;
                tx.registry
                    .maintenance_intents
                    .insert(selected.id.clone(), intent.reference.clone());
                tx.persist()?;
                guard(&tx)?;
                tx.verify_lock()?;
            }
            _ => return Err(conflict()),
        }
    } else if tx.registry.maintenance_intents.contains_key(&selected.id) {
        return Err(conflict());
    }
    guard(&tx)?;
    tx.verify_lock()?;
    Ok(intent)
}
fn unique<'a>(registry: &'a Registry, id: &str) -> Result<&'a Installation> {
    let mut found = registry.installations.iter().filter(|i| i.id == id);
    let item = found.next().ok_or_else(conflict)?;
    if found.next().is_some() {
        return Err(conflict());
    }
    Ok(item)
}
pub fn guard_start(
    store: &Store,
    refs: &References,
    registry: &Registry,
    current: &Instance,
) -> Result<()> {
    let Some(intent) = pending(store, refs, registry, current)? else {
        return Ok(());
    };
    let mut error = AppError::new(
        "VERSION_MAINTENANCE_PENDING",
        "Resume the recorded version maintenance; no original-version startup occurred.",
        4,
    );
    error.outcome = Some(crate::core::contracts::write::Outcome::Pending);
    error.next_action = Some(Box::new(match intent.operation() {
        Operation::Update => crate::core::contracts::action::NextAction::Update {
            instance: current.id.clone(),
            version: intent.target().version.clone(),
        },
        Operation::Rollback => crate::core::contracts::action::NextAction::Rollback {
            instance: current.id.clone(),
        },
    }));
    Err(error)
}
pub(crate) fn references_installation(
    store: &Store,
    refs: &References,
    registry: &Registry,
    current: &Instance,
    id: &str,
) -> Result<bool> {
    Ok(pending(store, refs, registry, current)?.is_some_and(|intent| intent.target().id == id))
}
pub(crate) fn conflict() -> AppError {
    AppError::new(
        "VERSION_MAINTENANCE_CONFLICT",
        "Original version maintenance is missing, changed or conflicts with current state; no replacement intent was inferred.",
        4,
    )
}

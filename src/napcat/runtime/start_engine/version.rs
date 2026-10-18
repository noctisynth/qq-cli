//! Switch an owned run to inspected target inputs; all native effects use existing capabilities.
use super::*;
mod rollback;
pub use rollback::{execute as rollback, target as rollback_target};

pub fn update<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    maintain_locked(
        context,
        selected,
        crate::napcat::runtime::version_intent::Operation::Update,
        deadline,
    )
}
fn maintain_locked<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    operation: crate::napcat::runtime::version_intent::Operation,
    deadline: Instant,
) -> Result<Started> {
    let registered = context.store.read()?;
    let current = crate::storage::state::select(&registered, Some(&selected.id))?;
    let inputs = crate::napcat::runtime::startup::inspect(
        context.store,
        &registered,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    if current.run_id == selected.run_id {
        if matches!(current.state.as_str(), "starting" | "unknown") {
            return Err(unknown_instance(&current.id, None));
        }
        if current.installation_id == inputs.installation().id {
            if matches!(selected.state.as_str(), "preparing" | "prepared")
                && matches!(
                    current.state.as_str(),
                    "preparing" | "prepared" | "ready" | "login_required"
                )
                && crate::napcat::runtime::instance_creation::transition::matches(
                    &current, selected,
                )
                && crate::napcat::runtime::generation::active(
                    context.store,
                    context.references,
                    &current,
                    deadline,
                )?
                .is_some()
            {
                return existing::locked(context, &current, deadline);
            }
            return existing::locked(context, selected, deadline);
        }
    }
    let intent = crate::napcat::runtime::version_intent::begin(
        context.store,
        context.references,
        selected,
        inputs.installation(),
        operation,
        deadline,
    )?;
    let source = intent.stopped_source();
    if current.run_id == selected.run_id {
        crate::napcat::runtime::stop::locked(
            context.store,
            context.references,
            selected,
            deadline,
        )?;
        if crate::storage::state::select(&context.store.read()?, Some(&selected.id))? != source {
            return Err(unknown_instance(&selected.id, None));
        }
    }
    let next = inputs.reserve_version_run(
        context.store,
        context.references,
        &source,
        &Evidence {
            release: context.release,
            witnesses: context.witnesses,
            probe: context.probe,
        },
        deadline,
    )?;
    launch::creation(context, &inputs, next.creation, &next.request_id, deadline)
}

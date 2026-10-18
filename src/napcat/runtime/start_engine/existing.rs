//! Existing instance selection and generation recovery, before the shared launch pipeline.
use super::*;
pub fn execute<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    locked(context, selected, deadline)
}
pub(super) fn locked<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let registered = context.store.read()?;
    let current = crate::storage::state::select(&registered, Some(&selected.id))?;
    crate::napcat::runtime::version_intent::guard_start(
        context.store,
        context.references,
        &registered,
        &current,
    )?;
    if selected.state == "stopped" {
        let inputs = crate::napcat::runtime::startup::inspect(
            context.store,
            &registered,
            context.release,
            context.witnesses,
            context.probe,
            deadline,
        )?;
        let next = inputs.reserve_next_run(
            context.store,
            context.references,
            selected,
            &Evidence {
                release: context.release,
                witnesses: context.witnesses,
                probe: context.probe,
            },
            deadline,
        )?;
        return launch::creation(context, &inputs, next.creation, &next.request_id, deadline);
    }
    if current != *selected {
        return Err(AppError::new(
            "START_INPUTS_CHANGED",
            "The selected instance changed before startup; no replacement was selected.",
            4,
        ));
    }
    match current.state.as_str() {
        "ready" | "login_required" => Ok(Started {
            instance: current,
            reused: true,
            request_id: None,
        }),
        "starting" | "unknown" => Err(unknown_instance(&current.id, None)),
        "preparing" | "prepared" => {
            if let Some((_, key)) = crate::napcat::runtime::generation::active(
                context.store,
                context.references,
                &current,
                deadline,
            )? {
                return explicit_locked(context, &key, deadline);
            }
            if registered.instances.len() == 1
                && let crate::napcat::runtime::default_start::Selection::Create { request_id } =
                    crate::napcat::runtime::default_start::select(
                        context.store,
                        context.references,
                        context.release.version,
                        deadline,
                    )?
            {
                return explicit_locked(context, &request_id, deadline);
            }
            Err(AppError::new(
                "INSTANCE_NOT_READY",
                "Resume the original creation request; no unsigned startup plan was inferred.",
                4,
            ))
        }
        _ => Err(crate::messaging::session::require_api_ready(&current)
            .err()
            .unwrap_or_else(AppError::storage)),
    }
}

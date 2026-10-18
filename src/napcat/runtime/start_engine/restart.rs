//! Restart an original run under one lifecycle lock; never stop an already-created successor.
use super::*;

pub fn execute<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    let registered = context.store.read()?;
    let current = crate::storage::state::select(&registered, Some(&selected.id))?;
    crate::napcat::runtime::version_intent::guard_start(
        context.store,
        context.references,
        &registered,
        &current,
    )?;
    // Inspect before a stop intent or native effect. Dispatch revalidates the same gates later.
    crate::napcat::runtime::startup::inspect(
        context.store,
        &registered,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    if current.run_id != selected.run_id {
        let mut source = selected.clone();
        source.state = "stopped".into();
        // The generation protocol checks the exact original stop proof, source and installation.
        return existing::locked(context, &source, deadline);
    }
    if matches!(current.state.as_str(), "starting" | "unknown") {
        return Err(unknown_instance(&current.id, None));
    }
    if matches!(selected.state.as_str(), "preparing" | "prepared")
        && crate::napcat::runtime::instance_creation::transition::matches(&current, selected)
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
    crate::napcat::runtime::stop::locked(context.store, context.references, selected, deadline)?;
    let stopped = crate::storage::state::select(&context.store.read()?, Some(&selected.id))?;
    let mut expected = selected.clone();
    expected.state = "stopped".into();
    if stopped != expected {
        return Err(unknown_instance(&selected.id, None));
    }
    existing::locked(context, &stopped, deadline)
}

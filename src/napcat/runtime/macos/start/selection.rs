//! Resolve original creation keys and reuse only their authenticated running instance.
use super::*;
use crate::storage::state::Registry;

pub(super) fn installation<'a>(
    registry: &'a Registry,
    instance: &Instance,
) -> Result<&'a crate::storage::state::Installation> {
    let found = registry
        .installations
        .iter()
        .filter(|installed| installed.id == instance.installation_id)
        .collect::<Vec<_>>();
    match found.as_slice() {
        [installed] => Ok(*installed),
        _ => Err(conflict()),
    }
}

pub(super) fn release(
    registry: &Registry,
    options: &Options<'_>,
) -> Result<crate::napcat::install::catalog::Release> {
    let version = match options.version {
        Some(version) => version,
        None if !options.new && !registry.instances.is_empty() => {
            let selected = crate::storage::state::select(registry, options.instance)?;
            return super::super::release::lookup(&installation(registry, &selected)?.version);
        }
        None => super::super::release::DEFAULT_VERSION,
    };
    super::super::release::lookup(version)
}
pub(super) fn request(
    context: &Context<'_, impl Credentials>,
    registry: &Registry,
    options: &Options<'_>,
    version: &str,
    deadline: Instant,
) -> Result<String> {
    let request = if options.new {
        options.request.ok_or_else(AppError::invalid)?.to_owned()
    } else if let Some(id) = options.instance {
        let instance = crate::storage::state::select(registry, Some(id))?;
        for_instance(context, &instance, deadline)?
    } else {
        if let [instance] = registry.instances.as_slice()
            && let Some((_, request)) = crate::napcat::runtime::generation::active(
                context.store,
                context.references,
                instance,
                deadline,
            )?
        {
            return Ok(request);
        }
        match crate::napcat::runtime::default_start::select_macos(
            context.store,
            context.references,
            version,
            deadline,
        )? {
            crate::napcat::runtime::default_start::Selection::Create { request_id } => request_id,
            crate::napcat::runtime::default_start::Selection::Existing(instance) => {
                for_instance(context, &instance, deadline)?
            }
        }
    };
    Ok(request)
}
pub(super) fn existing(
    context: &Context<'_, impl Credentials>,
    registry: &Registry,
    request: &str,
    version: &str,
    deadline: Instant,
) -> Result<Option<Started>> {
    if let Some(checkpoint) =
        super::super::creation::checkpoint(context.store, context.references, request)?
    {
        let current = crate::storage::state::select(registry, Some(&checkpoint.plan.instance_id))?;
        if current.run_id != checkpoint.plan.run_id || checkpoint.plan.version != version {
            return Err(conflict());
        }
        crate::napcat::runtime::version_intent::guard_start(
            context.store,
            context.references,
            registry,
            &current,
        )?;
        crate::napcat::runtime::generation::active(
            context.store,
            context.references,
            &current,
            deadline,
        )?;
        if matches!(
            current.state.as_str(),
            "ready" | "login_required" | "starting"
        ) {
            let installation = registry
                .installations
                .iter()
                .find(|v| v.id == current.installation_id)
                .ok_or_else(conflict)?;
            let tx = context.store.transaction(deadline)?;
            super::super::run::reopen_service(
                &tx,
                context.references,
                &current,
                installation,
                deadline,
            )?;
            drop(tx);
            return Ok(Some(Started {
                instance: current,
                reused: true,
                request: Some(request.to_owned()),
            }));
        }
        if !matches!(current.state.as_str(), "preparing" | "prepared") {
            return Err(unresolved());
        }
    }
    Ok(None)
}

fn for_instance(
    context: &Context<'_, impl Credentials>,
    instance: &Instance,
    deadline: Instant,
) -> Result<String> {
    if let Some((_, request)) = crate::napcat::runtime::generation::active(
        context.store,
        context.references,
        instance,
        deadline,
    )? {
        return Ok(request);
    }
    crate::napcat::runtime::instance_creation::macos_request(
        context.store,
        context.references,
        instance,
    )
}

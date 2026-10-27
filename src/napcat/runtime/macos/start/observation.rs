//! Reconstruct original launch inputs from authenticated records, without preparing effects.
use super::super::{adapter, creation, deploy, material, run};
use super::*;
use crate::{
    napcat::{install::installation, runtime::instance_creation},
    storage::state::Installation,
};

pub(crate) struct Observed {
    pub target: start_intent::Target,
    pub service: run::ServiceRun,
}

pub(crate) fn observe(
    context: &Context<'_, impl Credentials>,
    instance: &Instance,
    installed: &Installation,
    deadline: Instant,
) -> Result<Observed> {
    let request = instance_creation::macos_request(context.store, context.references, instance)?;
    let checkpoint =
        creation::checkpoint(context.store, context.references, &request)?.ok_or_else(conflict)?;
    if checkpoint.plan.instance_id != instance.id
        || checkpoint.plan.run_id != instance.run_id
        || checkpoint.installation_id != installed.id
        || checkpoint.installation_digest != installed.digest
    {
        return Err(conflict());
    }
    installation::verify(context.store, installed, context.references, deadline)?;
    let deployment =
        deploy::completed_target(context.store, context.references, instance, deadline)?
            .ok_or_else(conflict)?;
    let material = material::recover_running(
        context.store,
        context.references,
        context.credentials,
        instance,
        installed,
        deadline,
    )?;
    let adapter = adapter::Prepared::inspect_with_profile(
        material.directory(),
        &installed.directory,
        material.profile_home(),
    )?;
    adapter.verify_adapter()?;
    let launch = launch::Launch::new(
        prepare::Prepared {
            runtime: checkpoint.plan,
            deployment,
            material,
            adapter,
        },
        context,
        deadline,
    )?;
    let tx = context.store.transaction(deadline)?;
    let service = run::reopen_service(&tx, context.references, instance, installed, deadline)?;
    launch.verify_service(&service, deadline)?;
    tx.verify_lock()?;
    crate::core::deadline::check(deadline)?;
    Ok(Observed {
        target: launch.target,
        service,
    })
}

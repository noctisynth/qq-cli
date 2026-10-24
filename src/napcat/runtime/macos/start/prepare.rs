//! Compose existing reservation, material and assembly use cases; no process creation.
use super::super::{adapter, assembly, creation, deploy, layout, material};
use super::*;
use crate::napcat::{
    install::installation,
    runtime::{instance_creation::Creation, runtime_store::Plan},
};

pub struct Prepared {
    pub(super) runtime: Plan,
    pub(super) deployment: start_intent::Target,
    pub(super) material: material::Prepared,
    pub(super) adapter: adapter::Prepared,
}
pub fn prepare(
    context: &Context<'_, impl Credentials>,
    request: &str,
    deadline: Instant,
) -> Result<Prepared> {
    let version = creation::checkpoint(context.store, context.references, request)?
        .map(|checkpoint| checkpoint.plan.version)
        .unwrap_or_else(|| super::super::release::DEFAULT_VERSION.into());
    for_version(context, request, &version, deadline)
}
pub fn for_version(
    context: &Context<'_, impl Credentials>,
    request: &str,
    version: &str,
    deadline: Instant,
) -> Result<Prepared> {
    let release = super::super::release::lookup(version)?;
    let registered = context.store.read()?;
    let installations: Vec<_> = registered
        .installations
        .iter()
        .filter(|v| v.version == release.version)
        .collect();
    let installed = match installations.as_slice() {
        [installed] => (*installed).clone(),
        [] => {
            let mut error = AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Install the fixed macOS Framework release before starting QQ.",
                4,
            );
            error.next_action = Some(Box::new(
                crate::core::contracts::action::NextAction::Install {
                    version: release.version.into(),
                },
            ));
            return Err(error);
        }
        _ => {
            return Err(AppError::new(
                "INSTALLATION_AMBIGUOUS",
                "Multiple fixed installations match; no instance was prepared.",
                4,
            ));
        }
    };
    installation::verify(context.store, &installed, context.references, deadline)?;
    if let Some(checkpoint) = creation::checkpoint(context.store, context.references, request)? {
        if checkpoint.installation_id != installed.id
            || checkpoint.installation_digest != installed.digest
        {
            return Err(conflict());
        }
        let instance =
            crate::storage::state::select(&registered, Some(&checkpoint.plan.instance_id))?;
        if instance.run_id != checkpoint.plan.run_id || instance.installation_id != installed.id {
            return Err(conflict());
        }
        source::retire(context, request, deadline)?;
        if let Some(deployment) =
            deploy::completed_target(context.store, context.references, &instance, deadline)?
        {
            let material = materials(context, &checkpoint.plan, request, deadline)?;
            let adapter = adapter::Prepared::inspect(material.directory(), &installed.directory)?;
            return Ok(Prepared {
                runtime: checkpoint.plan,
                deployment,
                material,
                adapter,
            });
        }
    }
    source::with_source(context, request, deadline, |source, deadline| {
        let reservation = creation::reserve(
            context.store,
            context.references,
            source,
            &installed.id,
            request,
            deadline,
        )?;
        let runtime = match reservation {
            Creation::Fresh(reservation) => reservation.plan().clone(),
            Creation::Existing { instance, plan } => {
                if !matches!(instance.state.as_str(), "preparing" | "prepared") {
                    return Err(unresolved());
                }
                plan
            }
        };
        let layout::Preparation::Ready(_) = layout::prepare(
            context.store,
            context.references,
            &runtime.instance_id,
            request,
            deadline,
        )?
        else {
            return Err(unresolved());
        };
        let material = materials(context, &runtime, request, deadline)?;
        let assembly = assembly::Plan::inspect(
            context.store,
            context.references,
            source,
            &runtime.instance_id,
            deadline,
        )?;
        if assembly.deploy(context.store, context.references, request, deadline)?
            != deploy::Status::Complete
        {
            return Err(unresolved());
        }
        let adapter = adapter::Prepared::inspect(material.directory(), &installed.directory)?;
        Ok(Prepared {
            runtime,
            deployment: assembly.target().clone(),
            material,
            adapter,
        })
    })
}
fn materials(
    context: &Context<'_, impl Credentials>,
    runtime: &Plan,
    request: &str,
    deadline: Instant,
) -> Result<material::Prepared> {
    match material::prepare(
        context.store,
        context.references,
        context.credentials,
        runtime,
        request,
        deadline,
    )? {
        material::Preparation::Ready(v) => Ok(v),
        material::Preparation::Unknown => Err(unresolved()),
    }
}

impl Prepared {
    pub fn runtime(&self) -> &Plan {
        &self.runtime
    }
    pub fn executable(&self) -> std::path::PathBuf {
        self.adapter.executable()
    }
    /// Revalidate a prepared application without creating any process.
    pub fn verify(&self, context: &Context<'_, impl Credentials>, deadline: Instant) -> Result<()> {
        let tx = context.store.transaction(deadline)?;
        self.verify_in(&tx, context, deadline)
    }
    pub(super) fn verify_in(
        &self,
        tx: &crate::storage::state::Transaction<'_>,
        context: &Context<'_, impl Credentials>,
        deadline: Instant,
    ) -> Result<()> {
        deploy::verify_in(tx, context.references, &self.deployment, deadline)?;
        let installed = tx
            .registry
            .installations
            .iter()
            .find(|v| {
                v.id == self.deployment.installation_id
                    && v.digest == self.deployment.installation_digest
            })
            .ok_or_else(conflict)?;
        installation::verify(tx.store(), installed, context.references, deadline)?;
        material::verify_prelaunch_in(
            tx,
            context.references,
            context.credentials,
            &self.runtime,
            self.material.bundle(),
            deadline,
        )?;
        self.adapter.adapter_plan()?.verify()?;
        tx.verify_lock()
    }
}

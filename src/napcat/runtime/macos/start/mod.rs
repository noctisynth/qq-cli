//! macOS start use case: prepare a dedicated app, then dispatch one owned run.
use crate::{
    core::error::{AppError, Result},
    napcat::runtime::start_intent,
    storage::{
        credentials::Credentials,
        references::References,
        state::{Instance, Store},
    },
};
use sha2::{Digest, Sha256};
use std::time::Instant;
mod observation;
mod prepare;
mod selection;
mod source;
pub(super) use observation::observe;
pub use prepare::{Prepared, for_version as prepare_version, prepare as prepare_instance};

pub struct Context<'a, C: Credentials> {
    pub store: &'a Store,
    pub references: &'a References,
    pub credentials: &'a C,
}
fn conflict() -> AppError {
    AppError::new(
        "MACOS_START_CONFLICT",
        "The selected macOS instance or original startup inputs changed.",
        4,
    )
}
pub(super) fn unresolved() -> AppError {
    AppError::new(
        "START_OUTCOME_UNKNOWN",
        "An earlier preparation or startup is unresolved; inspect the original instance instead of repeating creation.",
        5,
    )
}

mod launch;
pub(super) mod sandbox;

pub struct Options<'a> {
    pub instance: Option<&'a str>,
    pub version: Option<&'a str>,
    pub request: Option<&'a str>,
    pub new: bool,
    pub verify_compatibility: bool,
}
pub struct Started {
    pub instance: Instance,
    pub reused: bool,
    pub request: Option<String>,
}
pub fn execute(
    context: &Context<'_, impl Credentials>,
    options: Options<'_>,
    deadline: Instant,
) -> Result<Started> {
    execute_with(context, options, None, deadline, prepare::for_version)
}
/// Same-install maintenance stays bound to the selected complete snapshot.
pub fn selected(
    context: &Context<'_, impl Credentials>,
    expected: &Instance,
    installation: &crate::storage::state::Installation,
    verify_compatibility: bool,
    deadline: Instant,
) -> Result<Started> {
    execute_with(
        context,
        Options {
            instance: Some(&expected.id),
            version: Some(&installation.version),
            request: None,
            new: false,
            verify_compatibility,
        },
        Some((expected, installation)),
        deadline,
        prepare::for_version,
    )
}
fn execute_with<C: Credentials>(
    context: &Context<'_, C>,
    options: Options<'_>,
    expected: Option<(&Instance, &crate::storage::state::Installation)>,
    deadline: Instant,
    prepare_run: impl FnOnce(&Context<'_, C>, &str, &str, Instant) -> Result<Prepared>,
) -> Result<Started> {
    if options.verify_compatibility && !options.new && expected.is_none() {
        return Err(AppError::invalid());
    }
    if let Some(version) = options.version {
        super::release::lookup(version)?;
    }
    let operation = crate::napcat::runtime::operation_lock::acquire(context.store, deadline)?;
    let registry = context.store.read()?;
    let release = selection::release(&registry, &options)?;
    if let Some((expected, installation)) = expected {
        let installed = registry
            .installations
            .iter()
            .filter(|entry| entry.id == expected.installation_id)
            .collect::<Vec<_>>();
        if crate::storage::state::select(&registry, Some(&expected.id))? != *expected
            || installed.as_slice() != [installation]
        {
            return Err(conflict());
        }
    }
    if !options.new && !registry.instances.is_empty() {
        let selected = crate::storage::state::select(&registry, options.instance)?;
        crate::napcat::runtime::version_intent::guard_start(
            context.store,
            context.references,
            &registry,
            &selected,
        )?;
        if selection::installation(&registry, &selected)?.version != release.version {
            return Err(AppError::new(
                "INSTANCE_VERSION_CONFLICT",
                "Requested version differs from the selected macOS installation.",
                4,
            ));
        }
        if selected.state == "stopped" {
            drop(operation);
            return super::restart::start_stopped(
                context,
                &selected,
                options.verify_compatibility,
                deadline,
            );
        }
    }
    let request = selection::request(context, &registry, &options, release.version, deadline)?;
    crate::core::domain::request_key(&request).map_err(|_| AppError::invalid())?;
    let result = (|| {
        if let Some(started) =
            selection::existing(context, &registry, &request, release.version, deadline)?
        {
            return Ok(started);
        }
        // A normal new start requires the exact software/environment combination
        // previously verified from an owned online dispatch.
        let known = super::compatibility::supported(
            context.store,
            context.references,
            &super::compatibility::Key::for_version(release.version)?,
        )?;
        if !options.verify_compatibility && !known {
            return Err(AppError::new(
                "COMPATIBILITY_UNKNOWN",
                "This macOS combination needs an explicit first startup trial: use napcat start --new --verify-compatibility --request-id KEY.",
                4,
            ));
        }
        let original_lock = operation.identity()?;
        // Preparation helpers own the same lifecycle lock. Release selection's
        // lock before composing them, then require the original lock identity again
        // for the final once-only dispatch; never recursively acquire it.
        drop(operation);
        let prepared = prepare_run(context, &request, release.version, deadline)?;
        let operation = crate::napcat::runtime::operation_lock::acquire(context.store, deadline)?;
        if operation.identity()? != original_lock {
            return Err(conflict());
        }
        operation.verify()?;
        let ports =
            crate::napcat::runtime::ports::Lease::acquire(prepared.runtime.ports, deadline)?;
        let launch = launch::Launch::new(prepared)?;
        let start_intent::Begin::Fresh(intent) = start_intent::begin(
            context.store,
            context.references,
            &request,
            &launch.target,
            deadline,
        )?
        else {
            return Err(unresolved());
        };
        let mut owner = None;
        let receipt = intent.dispatch(|tx| {
            operation.verify()?;
            drop(ports);
            owner = Some(launch.dispatch(tx, context, &request, deadline, &|| operation.verify())?);
            Ok(())
        })?;
        if receipt.outcome != start_intent::Outcome::Dispatched {
            return Err(unresolved());
        }
        let owner = owner.ok_or_else(unresolved)?;
        owner.verify(deadline)?;
        operation.verify()?;
        let mut tx = context.store.transaction(deadline)?;
        let instance = tx
            .registry
            .instances
            .iter_mut()
            .find(|v| v.id == launch.target.instance_id && v.run_id == launch.target.run_id)
            .ok_or_else(conflict)?;
        if instance.state != "starting" {
            return Err(conflict());
        }
        instance.state = "login_required".into();
        instance.process_id = Some(owner.identity().process_id());
        let instance = instance.clone();
        tx.commit()?;
        operation.verify()?;
        Ok(Started {
            instance,
            reused: false,
            request: Some(request.clone()),
        })
    })();
    result.map_err(|mut error: AppError| {
        if error.request_id.is_none() {
            error.request_id = Some(request);
        }
        error
    })
}

/// A recorded creation key fixes instance/run identity across the unlocked
/// preparation boundary. It cannot select a later successor by stable ID alone.
pub(super) fn resume(
    context: &Context<'_, impl Credentials>,
    expected: &Instance,
    request: &str,
    verify_compatibility: bool,
    deadline: Instant,
) -> Result<Started> {
    let checkpoint = super::creation::checkpoint(context.store, context.references, request)?
        .ok_or_else(conflict)?;
    if checkpoint.plan.instance_id != expected.id || checkpoint.plan.run_id != expected.run_id {
        return Err(conflict());
    }
    execute(
        context,
        Options {
            instance: None,
            version: Some(&checkpoint.plan.version),
            request: Some(request),
            new: true,
            verify_compatibility,
        },
        deadline,
    )
}

#[cfg(test)]
mod tests;

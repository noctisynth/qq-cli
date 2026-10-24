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
pub use prepare::{Prepared, prepare as prepare_instance};

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
fn unresolved() -> AppError {
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
    execute_with(context, options, deadline, prepare::prepare)
}
fn execute_with<C: Credentials>(
    context: &Context<'_, C>,
    options: Options<'_>,
    deadline: Instant,
    prepare_run: impl FnOnce(&Context<'_, C>, &str, Instant) -> Result<Prepared>,
) -> Result<Started> {
    if options.verify_compatibility && !options.new {
        return Err(AppError::invalid());
    }
    let release = crate::napcat::install::catalog::lookup(options.version.unwrap_or("4.18.41"))?;
    if release.version != "v4.18.41" {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "macOS startup currently requires the fixed Framework v4.18.41 release.",
            4,
        ));
    }
    let operation = crate::napcat::runtime::operation_lock::acquire(context.store, deadline)?;
    let registry = context.store.read()?;
    if !options.new && !registry.instances.is_empty() {
        let selected = crate::storage::state::select(&registry, options.instance)?;
        if selected.state == "stopped" {
            drop(operation);
            return super::restart::execute(context, &selected, deadline);
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
            &super::compatibility::Key::current()?,
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
        let prepared = prepare_run(context, &request, deadline)?;
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
            verify_compatibility: false,
        },
        deadline,
    )
}

#[cfg(test)]
mod tests;

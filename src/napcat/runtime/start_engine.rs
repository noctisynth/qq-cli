//! Explicit new-instance orchestration. Reviewed trust gates still precede all dispatch.
#![cfg(windows)]
mod existing;
mod launch;
use super::operation_lock::acquire as operation_lock;
use crate::{
    core::error::{AppError, Result},
    napcat::compatibility::Witness,
    napcat::doctor::Probe,
    napcat::install::catalog::Release,
    napcat::runtime::instance_creation::{Creation, Evidence},
    napcat::runtime::runtime_store,
    napcat::runtime::start_intent::{self, Begin, Outcome},
    storage::credentials::Credentials,
    storage::references::References,
    storage::state::{Instance, Store},
};
pub use existing::execute as selected;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::{Duration, Instant};

pub struct Context<'a, C: Credentials, P: Probe> {
    pub store: &'a Store,
    pub references: &'a References,
    pub credentials: &'a C,
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
    pub probe: &'a P,
    pub environment: &'a BTreeMap<OsString, OsString>,
}
pub struct Started {
    pub instance: Instance,
    pub reused: bool,
    pub request_id: Option<String>,
}

impl Started {
    pub(crate) fn stale(self) -> AppError {
        unknown_instance(&self.instance.id, self.request_id)
    }
}

pub fn explicit<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    request_id: &str,
    deadline: Instant,
) -> Result<Started> {
    crate::core::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
    let _operation = operation_lock(context.store, deadline)?;
    explicit_locked(context, request_id, deadline)
}

/// Recheck selection after acquiring the same operation lock used by explicit creation.
pub fn automatic<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    crate::napcat::runtime::startup::inspect(
        context.store,
        &context.store.read()?,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    let registered = context.store.read()?;
    if let [instance] = registered.instances.as_slice() {
        if instance.state == "stopped" {
            return existing::locked(context, instance, deadline);
        }
        if matches!(instance.state.as_str(), "preparing" | "prepared")
            && let Some((_, key)) = crate::napcat::runtime::generation::active(
                context.store,
                context.references,
                instance,
                deadline,
            )?
        {
            return explicit_locked(context, &key, deadline);
        }
    }
    match crate::napcat::runtime::default_start::select(
        context.store,
        context.references,
        context.release.version,
        deadline,
    )? {
        crate::napcat::runtime::default_start::Selection::Create { request_id } => {
            explicit_locked(context, &request_id, deadline)
        }
        crate::napcat::runtime::default_start::Selection::Existing(instance) => {
            match instance.state.as_str() {
                "ready" | "login_required" => Ok(Started {
                    instance,
                    reused: true,
                    request_id: None,
                }),
                "starting" | "unknown" => Err(unknown_instance(&instance.id, None)),
                _ => Err(crate::messaging::session::require_api_ready(&instance)
                    .err()
                    .unwrap_or_else(AppError::storage)),
            }
        }
    }
}

fn explicit_locked<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    request_id: &str,
    deadline: Instant,
) -> Result<Started> {
    let inputs = crate::napcat::runtime::startup::inspect(
        context.store,
        &context.store.read()?,
        context.release,
        context.witnesses,
        context.probe,
        deadline,
    )?;
    let evidence = Evidence {
        release: context.release,
        witnesses: context.witnesses,
        probe: context.probe,
    };
    let creation = inputs.reserve_instance(
        context.store,
        context.references,
        request_id,
        &evidence,
        deadline,
    )?;
    launch::creation(context, &inputs, creation, request_id, deadline)
}
fn unknown(request_id: &str, instance_id: &str) -> AppError {
    unknown_instance(instance_id, Some(request_id.into()))
}
fn unknown_instance(instance_id: &str, request_id: Option<String>) -> AppError {
    let mut error = AppError::new(
        "START_OUTCOME_UNKNOWN",
        "Startup may have been dispatched; inspect this instance and do not repeat creation with another key.",
        5,
    );
    error.outcome = Some("unknown");
    error.request_id = request_id;
    error.next_action = Some(Box::new(
        serde_json::json!({"command":"status","args":{"instance":instance_id}}),
    ));
    error
}

#[cfg(test)]
mod tests;

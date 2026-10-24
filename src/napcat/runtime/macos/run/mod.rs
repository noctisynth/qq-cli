//! Publish an original macOS dispatch and recover its authenticated service read-only.
use crate::core::error::{AppError, Result};
use crate::native::process::{Identity, Pinned};
use crate::storage::{
    references::References,
    state::{Installation, Instance, Transaction},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
mod journal;
mod publication;
mod scope;

use crate::native::macos_terminate::Target as SignalTarget;
use journal::authenticated;
pub use journal::has_record;
pub(super) use publication::publish;
pub(crate) use scope::Scope;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Platform {
    Macos,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    platform: Platform,
    request_id: String,
    target: crate::napcat::runtime::start_intent::Target,
    owner: Identity,
    endpoint: String,
    ws_endpoint: Option<String>,
    #[serde(default)]
    control_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility: Option<super::compatibility::Key>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signal: Option<SignalTarget>,
}
pub struct ServiceRun {
    owner: Pinned,
    control_endpoint: Option<String>,
    target: crate::napcat::runtime::start_intent::Target,
    compatibility: Option<super::compatibility::Key>,
    scope: Option<Scope>,
    audited: bool,
}
impl ServiceRun {
    pub(super) fn verifiable_inputs(&self) -> bool {
        self.scope.is_some() && self.audited
    }
    pub(super) fn scope(&self) -> Option<&Scope> {
        self.scope.as_ref()
    }
    pub(super) fn compatibility(&self) -> Option<&super::compatibility::Key> {
        self.compatibility.as_ref()
    }
    pub(super) fn target(&self) -> &crate::napcat::runtime::start_intent::Target {
        &self.target
    }
    pub fn control_endpoint(&self) -> Option<&str> {
        self.control_endpoint.as_deref()
    }
    pub fn owner(&self) -> &Pinned {
        &self.owner
    }
}
pub fn reopen_service(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
    let record = authenticated(tx, refs, instance, installation, deadline, true)?;
    let owner = match &record.signal {
        Some(signal) => signal.reopen(deadline)?,
        None => Pinned::reopen(&record.owner, deadline)?,
    };
    tx.verify_lock()?;
    crate::core::deadline::check(deadline)?;
    Ok(ServiceRun {
        owner,
        control_endpoint: record.control_endpoint,
        target: record.target,
        compatibility: record.compatibility,
        scope: record.scope,
        audited: record.signal.is_some(),
    })
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Origin {
    pub owner: Identity,
    pub signal: Option<SignalTarget>,
    pub scope: Option<Scope>,
    pub target: crate::napcat::runtime::start_intent::Target,
}
pub(super) fn stop_origin(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<Origin> {
    let record = authenticated(tx, refs, instance, installation, deadline, false)?;
    Ok(Origin {
        owner: record.owner,
        signal: record.signal,
        scope: record.scope,
        target: record.target,
    })
}
/// Authenticated launch metadata to publish with the original process.
pub(super) struct Binding<'a> {
    pub target: &'a crate::napcat::runtime::start_intent::Target,
    pub compatibility: &'a super::compatibility::Key,
    pub scope: &'a Scope,
    pub control_endpoint: &'a str,
}
fn conflict() -> AppError {
    AppError::new(
        "MACOS_RUN_CONFLICT",
        "The original macOS run, dispatch or service binding does not match; no endpoint was probed.",
        4,
    )
}

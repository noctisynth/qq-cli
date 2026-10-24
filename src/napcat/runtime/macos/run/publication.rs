//! Publish only the original dispatch callback process.
use super::journal::path;
use super::{Binding, Platform, Pointer, Record, SignalTarget, conflict};
use crate::{
    core::error::Result,
    native::process::Pinned,
    storage::{references::References, state::Transaction},
};
use std::time::Instant;

/// Persist only a process produced by this startup's dispatch callback. No
/// public PID/endpoint adoption entry exists; s1 must still confirm dispatch.
pub(in crate::napcat::runtime::macos) fn publish(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    owner: &Pinned,
    binding: Binding<'_>,
    deadline: Instant,
) -> Result<()> {
    let Binding {
        target,
        compatibility,
        scope,
        control_endpoint,
    } = binding;
    compatibility.validate()?;
    tx.verify_lock()?;
    owner.verify(deadline)?;
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|v| v.id == target.instance_id && v.run_id == target.run_id)
        .ok_or_else(conflict)?;
    if instance.state != "starting" || instance.installation_id != target.installation_id {
        return Err(conflict());
    }
    let intent = crate::napcat::runtime::start_intent::get(tx.store(), refs, request)?;
    if intent.target != *target
        || intent.outcome != crate::napcat::runtime::start_intent::Outcome::Pending
    {
        return Err(conflict());
    }
    crate::transport::client::Endpoint::parse(control_endpoint)?;
    let path = path(tx.store(), &instance.id, &instance.run_id)?;
    crate::storage::state::reject_links(&path)?;
    if path.try_exists().map_err(|_| conflict())? {
        return Err(conflict());
    }
    let record = Record {
        schema_version: 1,
        platform: Platform::Macos,
        request_id: request.into(),
        target: target.clone(),
        owner: owner.identity().clone(),
        endpoint: instance.endpoint.clone(),
        ws_endpoint: instance.ws_endpoint.clone(),
        control_endpoint: Some(control_endpoint.into()),
        compatibility: Some(compatibility.clone()),
        scope: Some(scope.clone()),
        signal: Some(SignalTarget::capture(owner, deadline)?),
    };
    crate::storage::state::private_dir(path.parent().ok_or_else(conflict)?)?;
    let reference = refs.issue("j2", &record)?;
    tx.verify_lock()?;
    owner.verify(deadline)?;
    crate::storage::state::atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )?;
    tx.verify_lock()?;
    owner.verify(deadline)
}

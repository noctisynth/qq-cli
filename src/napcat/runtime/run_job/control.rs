//! Recover the existing signed control binding; never discover, repair or retain a process.
use super::{
    Outcome, ServiceRun, conflict,
    journal::{load, path},
    unknown,
};
use crate::core::error::{AppError, Result};
use crate::native::{job::Job, process::Pinned};
use crate::storage::{
    references::References,
    state::{Installation, Instance, Transaction},
};
use std::time::Instant;

pub struct ControlRun {
    service: ServiceRun,
    endpoint: String,
}
impl ControlRun {
    pub fn service(&self) -> &ServiceRun {
        &self.service
    }
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
pub fn reopen_control(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<ControlRun> {
    crate::core::deadline::check(deadline)?;
    if tx
        .registry
        .instances
        .iter()
        .filter(|i| i.id == instance.id)
        .count()
        != 1
        || !tx.registry.instances.iter().any(|i| i == instance)
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == installation.id)
            .count()
            != 1
        || !tx.registry.installations.iter().any(|i| i == installation)
        || instance.installation_id != installation.id
        || !matches!(
            instance.state.as_str(),
            "starting" | "login_required" | "ready"
        )
    {
        return Err(conflict());
    }
    let record = load(&path(tx.store(), &instance.id, &instance.run_id)?, refs)?;
    if record.outcome != Outcome::Spawned
        || record.target.instance_id != instance.id
        || record.target.run_id != instance.run_id
        || record.target.installation_id != installation.id
        || record.target.installation_digest != installation.digest
    {
        return Err(conflict());
    }
    let unavailable = || {
        AppError::new(
            "CONTROL_NOT_BOUND",
            "This run has no existing signed control binding; no endpoint was probed.",
            4,
        )
    };
    let expected = record.discovery.as_ref().ok_or_else(unavailable)?;
    let endpoint = expected
        .control_endpoint
        .as_deref()
        .ok_or_else(unavailable)?;
    let control = record.prelogin.as_ref().ok_or_else(unavailable)?;
    if expected.endpoint != instance.endpoint
        || control.endpoint != endpoint
        || endpoint == expected.endpoint
        || control.owner.image() != expected.image
        || record
            .service
            .as_ref()
            .is_some_and(|s| s.owner != control.owner || s.endpoint != expected.endpoint)
    {
        return Err(conflict());
    }
    crate::transport::client::Endpoint::parse(endpoint)?;
    let job = Job::reopen(&record.job, deadline)?;
    let owner = Pinned::reopen(&control.owner, deadline)?;
    if !job.contains(&owner, deadline)? {
        return Err(conflict());
    }
    let retained = crate::napcat::runtime::retention::existing(
        tx,
        refs,
        &instance.id,
        &instance.run_id,
        &job,
        &owner,
        deadline,
    )?;
    if retained.outcome() != crate::napcat::runtime::retention::Outcome::Retained {
        return Err(unknown());
    }
    crate::core::deadline::check(deadline)?;
    Ok(ControlRun {
        service: ServiceRun { job, owner },
        endpoint: endpoint.into(),
    })
}

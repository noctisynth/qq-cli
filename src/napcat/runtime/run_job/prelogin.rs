use super::journal::{load, path, save};
use super::{Outcome, Service, ServiceRun, conflict, unknown};
use crate::core::error::{AppError, Result};
use crate::napcat::runtime::start_intent::Target;
use crate::native::job::Job;
use crate::native::process::Pinned;
use crate::storage::{references::References, state::Transaction};
use std::time::Instant;
/// Preserve the Job in the verified pre-login control process before the CLI
/// releases its last local handle. No control protocol, token or API data is sent. The later
/// account API must be hosted by this same complete process identity.
pub fn retain_prelogin(
    tx: &Transaction<'_>,
    refs: &References,
    target: &Target,
    job: &Job,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::core::deadline::check(deadline)?;
    let pointer = path(tx.store(), &target.instance_id, &target.run_id)?;
    let mut record = load(&pointer, refs)?;
    if record.target != *target
        || record.outcome != Outcome::Spawned
        || record.job != *job.id()
        || !tx.registry.instances.iter().any(|i| {
            i.id == target.instance_id
                && i.run_id == target.run_id
                && i.installation_id == target.installation_id
                && matches!(i.state.as_str(), "starting" | "login_required" | "ready")
        })
        || !tx
            .registry
            .installations
            .iter()
            .any(|i| i.id == target.installation_id && i.digest == target.installation_digest)
    {
        return Err(conflict());
    }
    let expected = record.discovery.as_ref().ok_or_else(conflict)?;
    let endpoint = expected.control_endpoint.as_deref().ok_or_else(|| {
        AppError::new(
            "PRELOGIN_RETENTION_UNAVAILABLE",
            "The signed run has no pre-login retention endpoint; no endpoint was probed.",
            4,
        )
    })?;
    if endpoint == expected.endpoint {
        return Err(conflict());
    }
    if let Some(control) = &record.prelogin {
        if control.endpoint != endpoint || control.owner.image() != expected.image {
            return Err(conflict());
        }
        let owner = Pinned::reopen(&control.owner, deadline)?;
        if !job.contains(&owner, deadline)? {
            return Err(conflict());
        }
        let retained = crate::napcat::runtime::retention::existing(
            tx,
            refs,
            &target.instance_id,
            &target.run_id,
            job,
            &owner,
            deadline,
        )?;
        if retained.outcome() != crate::napcat::runtime::retention::Outcome::Retained {
            return Err(unknown());
        }
        return Ok(ServiceRun {
            job: Job::reopen(job.id(), deadline)?,
            owner,
        });
    }
    let endpoint = crate::transport::client::Endpoint::parse(endpoint)?;
    let address = endpoint
        .as_str()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .map_err(|_| conflict())?;
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(conflict)?;
    let stream = std::net::TcpStream::connect_timeout(
        &address,
        remaining.min(std::time::Duration::from_secs(2)),
    )
    .map_err(|_| {
        AppError::new(
            "SERVICE_NOT_READY",
            "Pre-login control endpoint is not ready; no protocol bytes were sent.",
            4,
        )
        .safe_to_retry()
    })?;
    let (bound, owner) = job.identify_connected(stream, &expected.image, deadline)?;
    if record
        .service
        .as_ref()
        .is_some_and(|s| s.owner != *owner.identity())
    {
        return Err(conflict());
    }
    let _stream = job.bind_connected(bound.into_stream(), &owner, deadline)?;
    let retained = crate::napcat::runtime::retention::retain_once(
        tx,
        refs,
        &target.instance_id,
        &target.run_id,
        job,
        &owner,
        deadline,
    )?;
    if retained.outcome() != crate::napcat::runtime::retention::Outcome::Retained {
        return Err(unknown());
    }
    owner.verify(deadline)?;
    if !job.contains(&owner, deadline)? {
        return Err(conflict());
    }
    record.prelogin = Some(Service {
        owner: owner.identity().clone(),
        endpoint: endpoint.as_str().into(),
    });
    save(&pointer, refs, &record).map_err(|_| unknown())?;
    Ok(ServiceRun {
        job: Job::reopen(job.id(), deadline)?,
        owner,
    })
}

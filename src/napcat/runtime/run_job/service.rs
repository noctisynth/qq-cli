use super::journal::{load, path, save};
use super::{Outcome, Service, ServiceRun, conflict, unknown};
use crate::core::error::{AppError, Result};
use crate::napcat::runtime::start_intent::Target;
use crate::native::job::{BoundStream, Job};
use crate::native::process::Pinned;
use crate::storage::{
    references::References,
    state::{Store, Transaction},
};
use std::time::Instant;
/// Commit the actual HTTP service from a verified stream, retaining its Job first.
/// This does not mark an account logged in, an API ready, or startup trust gates satisfied.
pub fn bind_service(
    tx: &Transaction<'_>,
    refs: &References,
    target: &Target,
    job: &Job,
    bound: BoundStream,
    owner: &Pinned,
    deadline: Instant,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|i| {
            i.id == target.instance_id
                && i.run_id == target.run_id
                && i.installation_id == target.installation_id
        })
        .ok_or_else(conflict)?;
    if !tx
        .registry
        .installations
        .iter()
        .any(|i| i.id == target.installation_id && i.digest == target.installation_digest)
        || !matches!(
            instance.state.as_str(),
            "starting" | "login_required" | "ready"
        )
    {
        return Err(conflict());
    }
    let path = path(tx.store(), &target.instance_id, &target.run_id)?;
    let mut record = load(&path, refs)?;
    if record.target != *target
        || record.outcome != Outcome::Spawned
        || &record.job != job.id()
        || bound.owner() != owner.identity()
    {
        return Err(conflict());
    }
    let endpoint = crate::transport::client::Endpoint::parse(&instance.endpoint)?;
    if record.discovery.as_ref().is_some_and(|expected| {
        expected.image != owner.identity().image() || expected.endpoint != endpoint.as_str()
    }) {
        return Err(conflict());
    }
    if record
        .prelogin
        .as_ref()
        .is_some_and(|control| control.owner != *owner.identity())
    {
        return Err(conflict());
    }
    let address: std::net::SocketAddr = endpoint
        .as_str()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .parse()
        .map_err(|_| AppError::invalid())?;
    let stream = bound.into_stream();
    if stream.peer_addr().map_err(|_| conflict())? != address {
        return Err(conflict());
    }
    if record
        .service
        .as_ref()
        .is_some_and(|s| s.owner != *owner.identity() || s.endpoint != endpoint.as_str())
    {
        return Err(conflict());
    }
    // Recheck the held socket immediately before retention/publication, not a new connection.
    let _stream = job.bind_connected(stream, owner, deadline)?;
    let retention = if record.service.is_some() || record.prelogin.is_some() {
        crate::napcat::runtime::retention::existing
    } else {
        crate::napcat::runtime::retention::retain_once
    };
    let retained = retention(
        tx,
        refs,
        &target.instance_id,
        &target.run_id,
        job,
        owner,
        deadline,
    )?;
    if retained.outcome() != crate::napcat::runtime::retention::Outcome::Retained {
        return Err(unknown());
    }
    owner.verify(deadline)?;
    if !job.contains(owner, deadline)? {
        return Err(conflict());
    }
    if record.service.is_none() {
        record.service = Some(Service {
            owner: owner.identity().clone(),
            endpoint: endpoint.as_str().into(),
        });
        save(&path, refs, &record).map_err(|_| unknown())?;
    }
    crate::core::deadline::check(deadline).map_err(|_| unknown())
}

/// One zero-byte connection to the signed planned endpoint, no credentials or API.
/// Existing bindings recover exactly; legacy journals cannot guess missing inputs.
pub fn discover_service(
    tx: &Transaction<'_>,
    refs: &References,
    target: &Target,
    job: &Job,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::core::deadline::check(deadline)?;
    let record = load(
        &path(tx.store(), &target.instance_id, &target.run_id)?,
        refs,
    )?;
    if record.target != *target || record.outcome != Outcome::Spawned || record.job != *job.id() {
        return Err(conflict());
    }
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|item| {
            item.id == target.instance_id
                && item.run_id == target.run_id
                && item.installation_id == target.installation_id
        })
        .ok_or_else(conflict)?;
    let installation = tx
        .registry
        .installations
        .iter()
        .find(|item| item.id == target.installation_id && item.digest == target.installation_digest)
        .ok_or_else(conflict)?;
    if !matches!(
        instance.state.as_str(),
        "starting" | "login_required" | "ready"
    ) {
        return Err(conflict());
    }
    if record.service.is_some() {
        return reopen_service(tx.store(), refs, instance, installation, deadline);
    }
    let expected = record.discovery.as_ref().ok_or_else(|| {
        AppError::new(
            "SERVICE_DISCOVERY_UNAVAILABLE",
            "This signed run has no recorded service discovery inputs; no endpoint was probed.",
            4,
        )
    })?;
    if !expected.image.is_absolute()
        || expected.endpoint
            != crate::transport::client::Endpoint::parse(&instance.endpoint)?.as_str()
    {
        return Err(conflict());
    }
    let address = expected
        .endpoint
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
            "The planned service is not connectable yet; no API request was sent.",
            4,
        )
        .safe_to_retry()
    })?;
    let (bound, owner) = job.identify_connected(stream, &expected.image, deadline)?;
    bind_service(tx, refs, target, job, bound, &owner, deadline)?;
    reopen_service(tx.store(), refs, instance, installation, deadline)
}

/// Recover the signed Job for a pending selected instance; never enumerate or
/// accept a replacement process/image/endpoint from the caller.
pub fn discover_registered(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &crate::storage::state::Instance,
    installation: &crate::storage::state::Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::core::deadline::check(deadline)?;
    if !matches!(instance.state.as_str(), "starting" | "login_required")
        || !tx.registry.instances.iter().any(|item| item == instance)
        || !tx
            .registry
            .installations
            .iter()
            .any(|item| item == installation)
        || instance.installation_id != installation.id
    {
        return Err(conflict());
    }
    let record = load(&path(tx.store(), &instance.id, &instance.run_id)?, refs)?;
    if record.target.instance_id != instance.id
        || record.target.run_id != instance.run_id
        || record.target.installation_id != installation.id
        || record.target.installation_digest != installation.digest
        || record.outcome != Outcome::Spawned
    {
        return Err(conflict());
    }
    if record.service.is_some() {
        return reopen_service(tx.store(), refs, instance, installation, deadline);
    }
    if record.discovery.is_none() {
        return Err(AppError::new(
            "SERVICE_DISCOVERY_UNAVAILABLE",
            "This signed run has no recorded discovery inputs; no endpoint was probed.",
            4,
        ));
    }
    let job = Job::reopen(&record.job, deadline)?;
    discover_service(tx, refs, &record.target, &job, deadline)
}

/// Recover exactly the signed service of the selected registered run and installation.
/// Credential access and all HTTP/WS traffic happen later, using these native capabilities.
pub fn reopen_service(
    store: &Store,
    refs: &References,
    instance: &crate::storage::state::Instance,
    installation: &crate::storage::state::Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::core::deadline::check(deadline)?;
    let record = load(&path(store, &instance.id, &instance.run_id)?, refs)?;
    if record.outcome != Outcome::Spawned
        || record.target.instance_id != instance.id
        || record.target.run_id != instance.run_id
        || instance.installation_id != installation.id
        || record.target.installation_id != installation.id
        || record.target.installation_digest != installation.digest
    {
        return Err(conflict());
    }
    let service = record.service.ok_or_else(|| {
        AppError::new(
            "SERVICE_NOT_BOUND",
            "The signed run has no verified service binding yet.",
            4,
        )
    })?;
    if record
        .prelogin
        .as_ref()
        .is_some_and(|control| control.owner != service.owner)
    {
        return Err(conflict());
    }
    if record.discovery.as_ref().is_some_and(|expected| {
        expected.image != service.owner.image() || expected.endpoint != service.endpoint
    }) {
        return Err(conflict());
    }
    if service.endpoint != crate::transport::client::Endpoint::parse(&instance.endpoint)?.as_str() {
        return Err(conflict());
    }
    let owner = Pinned::reopen(&service.owner, deadline)?;
    let job = Job::reopen(&record.job, deadline)?;
    if !job.contains(&owner, deadline)? {
        return Err(conflict());
    }
    Ok(ServiceRun { job, owner })
}

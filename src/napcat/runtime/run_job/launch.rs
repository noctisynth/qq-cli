use super::journal::{load, path, save};
use super::{
    Discovery, Outcome, Record, Running, ServiceExpectation, conflict, process_image_path, unknown,
};
use crate::core::error::Result;
use crate::napcat::runtime::start_intent;
use crate::napcat::runtime::start_intent::Target;
use crate::native::job::{Job, SpawnSpec};
use crate::native::process::Pinned;
use crate::storage::{
    references::References,
    state::{Store, Transaction, reject_links},
};
use std::time::Instant;
/// Call only inside a fresh start_intent dispatch, after all trust gates.
/// A prior journal, including pending/unknown, never authorizes a second spawn.
pub fn launch_once(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    target: &Target,
    spec: &SpawnSpec<'_>,
    deadline: Instant,
) -> Result<Running> {
    launch_recorded(tx, refs, request, target, (spec, None, None), deadline)
}
/// Expected service inputs are persisted before CreateProcess, never accepted
/// from a later discovery call or inferred from an untrusted endpoint response.
pub fn launch_for_service_once(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    target: &Target,
    spec: &SpawnSpec<'_>,
    expected: &ServiceExpectation<'_>,
    deadline: Instant,
) -> Result<Running> {
    crate::core::deadline::check(deadline)?;
    let service_image = expected.image;
    process_image_path(service_image)?;
    reject_links(service_image)?;
    if !service_image.is_absolute() || !service_image.is_file() {
        return Err(conflict());
    }
    let ports = crate::napcat::runtime::ports::Lease::acquire(expected.ports, deadline)?;
    launch_recorded(
        tx,
        refs,
        request,
        target,
        (spec, Some(expected), Some(ports)),
        deadline,
    )
}
fn launch_recorded(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    target: &Target,
    launch: (
        &SpawnSpec<'_>,
        Option<&ServiceExpectation<'_>>,
        Option<crate::napcat::runtime::ports::Lease>,
    ),
    deadline: Instant,
) -> Result<Running> {
    let (spec, service_expectation, ports) = launch;
    crate::core::deadline::check(deadline)?;
    let intent = start_intent::get(tx.store(), refs, request)?;
    if intent.outcome != start_intent::Outcome::Pending || &intent.target != target {
        return Err(conflict());
    }
    let instance = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == target.instance_id)
        .ok_or_else(conflict)?;
    if instance.run_id != target.run_id
        || instance.installation_id != target.installation_id
        || instance.state != "starting"
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
        || !tx
            .registry
            .installations
            .iter()
            .any(|i| i.id == target.installation_id && i.digest == target.installation_digest)
    {
        return Err(conflict());
    }
    if let Some(expected) = service_expectation
        && crate::transport::client::Endpoint::parse(expected.control_endpoint)?.as_str()
            == crate::transport::client::Endpoint::parse(&instance.endpoint)?.as_str()
    {
        return Err(conflict());
    }
    let path = path(tx.store(), &target.instance_id, &target.run_id)?;
    match load(&path, refs) {
        Ok(_) => return Err(conflict()),
        Err(e) if e.code == "FILE_NOT_FOUND" => (),
        Err(e) => return Err(e),
    }
    if let Some(expected) = service_expectation
        && (instance.endpoint != format!("http://127.0.0.1:{}/", expected.ports.http)
            || instance.ws_endpoint.as_deref()
                != Some(format!("ws://127.0.0.1:{}/event", expected.ports.events).as_str())
            || expected.control_endpoint != format!("http://127.0.0.1:{}/", expected.ports.control))
    {
        return Err(conflict());
    }
    let job = Job::create(deadline)?;
    let mut record = Record {
        schema_version: 1,
        request_id: request.into(),
        target: target.clone(),
        job: job.id().clone(),
        outcome: Outcome::Pending,
        root: None,
        service: None,
        prelogin: None,
        discovery: service_expectation
            .map(|expected| {
                Ok(Discovery {
                    // QQ metadata uses canonical extended local paths, while the Win32
                    // process image query returns the ordinary drive path. Strip only
                    // that local-drive prefix; never normalize UNC, case or aliases.
                    image: process_image_path(expected.image)?,
                    endpoint: crate::transport::client::Endpoint::parse(&instance.endpoint)?
                        .as_str()
                        .to_owned(),
                    control_endpoint: Some(
                        crate::transport::client::Endpoint::parse(expected.control_endpoint)?
                            .as_str()
                            .to_owned(),
                    ),
                })
            })
            .transpose()?,
    };
    // This is the only durable source of the random Job name, before CreateProcess.
    save(&path, refs, &record)?;
    // Release only after durable intent, immediately before native creation.
    // Upstream cannot inherit the listeners; subsequent ownership checks remain mandatory.
    drop(ports);
    let spawned = match job.spawn(spec, deadline) {
        Ok(value) => value,
        Err(error) => {
            #[cfg(test)]
            eprintln!("synthetic job spawn failed: {error:?}");
            let _ = error;
            record.outcome = Outcome::Unknown;
            let _ = save(&path, refs, &record);
            return Err(unknown());
        }
    };
    let root = spawned.into_process();
    record.outcome = Outcome::Spawned;
    record.root = Some(root.identity().clone());
    save(&path, refs, &record).map_err(|error| {
        #[cfg(test)]
        eprintln!("synthetic post-spawn journal save failed: {error:?}");
        let _ = error;
        unknown()
    })?;
    crate::core::deadline::check(deadline).map_err(|_| unknown())?;
    Ok(Running { job, root })
}
/// Reopens only the signed original root, never searches for a replacement process.
/// An exited loader cannot be recovered through this root-only entry point.
pub fn reopen_root(
    store: &Store,
    refs: &References,
    target: &Target,
    deadline: Instant,
) -> Result<Running> {
    crate::core::deadline::check(deadline)?;
    let record = load(&path(store, &target.instance_id, &target.run_id)?, refs)?;
    if record.target != *target || record.outcome != Outcome::Spawned {
        return Err(conflict());
    }
    let root = Pinned::reopen(record.root.as_ref().ok_or_else(conflict)?, deadline)?;
    let job = Job::reopen(&record.job, deadline)?;
    if !job.contains(&root, deadline)? {
        return Err(conflict());
    }
    Ok(Running { job, root })
}

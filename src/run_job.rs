//! Durable native dispatch under the startup transaction. Not a license or compatibility gate.
use crate::config::read_json;
use crate::error::{AppError, Result};
use crate::native_job::{BoundStream, Job, JobId, SpawnSpec};
use crate::process::{Identity, Pinned};
use crate::references::References;
use crate::start_intent::{self, Target};
use crate::state::{Store, Transaction, atomic_json, private_dir, reject_links};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Pending,
    Spawned,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    target: Target,
    job: JobId,
    outcome: Outcome,
    root: Option<Identity>,
    #[serde(default)]
    service: Option<Service>,
    #[serde(default)]
    discovery: Option<Discovery>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Discovery {
    image: PathBuf,
    endpoint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Service {
    owner: Identity,
    endpoint: String,
}
/// Verified service capability. Contains private process paths and must not be serialized.
pub struct ServiceRun {
    job: Job,
    owner: Pinned,
}
impl ServiceRun {
    pub fn job(&self) -> &Job {
        &self.job
    }
    pub fn owner(&self) -> &Pinned {
        &self.owner
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
/// Owns the observed root and Job. Dropping does not terminate either process or Job members.
pub struct Running {
    job: Job,
    root: Pinned,
}
impl Running {
    pub fn job(&self) -> &Job {
        &self.job
    }
    pub fn root(&self) -> &Pinned {
        &self.root
    }
}
fn path(store: &Store, instance: &str, run: &str) -> Result<PathBuf> {
    crate::domain::request_key(instance).map_err(|_| AppError::invalid())?;
    crate::domain::request_key(run).map_err(|_| AppError::invalid())?;
    let digest =
        Sha256::digest(serde_json::to_vec(&(instance, run)).map_err(|_| AppError::storage())?);
    Ok(store
        .root()
        .join("run-jobs")
        .join(format!("{digest:x}.json")))
}
/// Presence is only a routing signal, never validation or permission to access an API.
pub fn has_record(store: &Store, instance: &str, run: &str) -> Result<bool> {
    let path = path(store, instance, run)?;
    reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
fn save(path: &Path, refs: &References, record: &Record) -> Result<()> {
    reject_links(path)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    let reference = refs.issue("j1", record)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}
fn load(path: &Path, refs: &References) -> Result<Record> {
    reject_links(path)?;
    let pointer: Pointer = read_json(path, 4096)?;
    if pointer.schema_version != 1 {
        return Err(AppError::storage());
    }
    let record: Record = refs.get("j1", &pointer.reference)?;
    record.job.validate()?;
    crate::domain::request_key(&record.request_id).map_err(|_| AppError::storage())?;
    if record.schema_version != 1
        || (record.outcome == Outcome::Spawned) != record.root.is_some()
        || (record.service.is_some() && record.outcome != Outcome::Spawned)
    {
        return Err(AppError::storage());
    }
    Ok(record)
}
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
    launch_recorded(tx, refs, request, target, (spec, None), deadline)
}
/// Expected service inputs are persisted before CreateProcess, never accepted
/// from a later discovery call or inferred from an untrusted endpoint response.
pub fn launch_for_service_once(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    target: &Target,
    spec: &SpawnSpec<'_>,
    service_image: &Path,
    deadline: Instant,
) -> Result<Running> {
    crate::deadline::check(deadline)?;
    process_image_path(service_image)?;
    reject_links(service_image)?;
    if !service_image.is_absolute() || !service_image.is_file() {
        return Err(conflict());
    }
    launch_recorded(
        tx,
        refs,
        request,
        target,
        (spec, Some(service_image)),
        deadline,
    )
}
fn launch_recorded(
    tx: &Transaction<'_>,
    refs: &References,
    request: &str,
    target: &Target,
    launch: (&SpawnSpec<'_>, Option<&Path>),
    deadline: Instant,
) -> Result<Running> {
    let (spec, service_image) = launch;
    crate::deadline::check(deadline)?;
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
    let path = path(tx.store(), &target.instance_id, &target.run_id)?;
    match load(&path, refs) {
        Ok(_) => return Err(conflict()),
        Err(e) if e.code == "FILE_NOT_FOUND" => (),
        Err(e) => return Err(e),
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
        discovery: service_image
            .map(|image| {
                Ok(Discovery {
                    // QQ metadata uses canonical extended local paths, while the Win32
                    // process image query returns the ordinary drive path. Strip only
                    // that local-drive prefix; never normalize UNC, case or aliases.
                    image: process_image_path(image)?,
                    endpoint: crate::client::Endpoint::parse(&instance.endpoint)?
                        .as_str()
                        .to_owned(),
                })
            })
            .transpose()?,
    };
    // This is the only durable source of the random Job name, before CreateProcess.
    save(&path, refs, &record)?;
    let spawned = match job.spawn(spec, deadline) {
        Ok(value) => value,
        Err(_) => {
            record.outcome = Outcome::Unknown;
            let _ = save(&path, refs, &record);
            return Err(unknown());
        }
    };
    let root = spawned.into_process();
    record.outcome = Outcome::Spawned;
    record.root = Some(root.identity().clone());
    save(&path, refs, &record).map_err(|_| unknown())?;
    crate::deadline::check(deadline).map_err(|_| unknown())?;
    Ok(Running { job, root })
}
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
    crate::deadline::check(deadline)?;
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
    let endpoint = crate::client::Endpoint::parse(&instance.endpoint)?;
    if record.discovery.as_ref().is_some_and(|expected| {
        expected.image != owner.identity().image() || expected.endpoint != endpoint.as_str()
    }) {
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
    let retention = if record.service.is_some() {
        crate::retention::existing
    } else {
        crate::retention::retain_once
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
    if retained.outcome() != crate::retention::Outcome::Retained {
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
    crate::deadline::check(deadline).map_err(|_| unknown())
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
    crate::deadline::check(deadline)?;
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
        || expected.endpoint != crate::client::Endpoint::parse(&instance.endpoint)?.as_str()
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
    instance: &crate::state::Instance,
    installation: &crate::state::Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::deadline::check(deadline)?;
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
    instance: &crate::state::Instance,
    installation: &crate::state::Installation,
    deadline: Instant,
) -> Result<ServiceRun> {
    crate::deadline::check(deadline)?;
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
    if record.discovery.as_ref().is_some_and(|expected| {
        expected.image != service.owner.image() || expected.endpoint != service.endpoint
    }) {
        return Err(conflict());
    }
    if service.endpoint != crate::client::Endpoint::parse(&instance.endpoint)?.as_str() {
        return Err(conflict());
    }
    let owner = Pinned::reopen(&service.owner, deadline)?;
    let job = Job::reopen(&record.job, deadline)?;
    if !job.contains(&owner, deadline)? {
        return Err(conflict());
    }
    Ok(ServiceRun { job, owner })
}
/// Reopens only the signed original root, never searches for a replacement process.
/// An exited loader cannot be recovered through this root-only entry point.
pub fn reopen_root(
    store: &Store,
    refs: &References,
    target: &Target,
    deadline: Instant,
) -> Result<Running> {
    crate::deadline::check(deadline)?;
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
fn conflict() -> AppError {
    AppError::new(
        "RUN_JOB_CONFLICT",
        "The run's durable dispatch or original process does not match; no process was created.",
        4,
    )
}
fn process_image_path(path: &Path) -> Result<PathBuf> {
    let text = path.to_str().ok_or_else(conflict)?;
    if text.chars().any(char::is_control) {
        return Err(conflict());
    }
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        let bytes = rest.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'\\'
        {
            return Err(conflict());
        }
        return Ok(rest.into());
    }
    let bytes = text.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return Err(conflict());
    }
    Ok(path.to_owned())
}
fn unknown() -> AppError {
    let mut e = AppError::new(
        "START_OUTCOME_UNKNOWN",
        "Native dispatch may have occurred; do not repeat it automatically.",
        5,
    );
    e.outcome = Some("unknown");
    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::start_intent::Begin;
    use crate::state::{Installation, Instance};
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::time::Duration;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }
    fn target() -> Target {
        Target {
            instance_id: "original-instance".into(),
            run_id: "original-run".into(),
            installation_id: "original-installation".into(),
            installation_digest: "a".repeat(64),
            input_hash: "b".repeat(64),
        }
    }
    fn setup(store: &Store) {
        let t = target();
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.installations.push(Installation {
            id: t.installation_id.clone(),
            version: "v0.0.0".into(),
            digest: t.installation_digest,
            directory: store.root().join("unused-installation"),
        });
        tx.registry.instances.push(Instance {
            id: t.instance_id,
            installation_id: t.installation_id,
            endpoint: "http://127.0.0.1:1/".into(),
            ws_endpoint: None,
            credential_key: "unused-original".into(),
            account: None,
            run_id: t.run_id,
            state: "prepared".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
    }
    #[test]
    fn signed_job_precedes_original_child_and_root_recovers_without_respawn() {
        const ROLE: &str = "QQ_CLI_TEST_RUN_JOB_ROOT";
        const NAME: &str =
            "run_job::tests::signed_job_precedes_original_child_and_root_recovers_without_respawn";
        if let Some(root) = std::env::var_os(ROLE) {
            let root = PathBuf::from(root);
            let store = Store::new(root.join("state"));
            let refs = References::with_key(store.root().join("references"), [47; 32]);
            let t = target();
            let record = load(&path(&store, &t.instance_id, &t.run_id).unwrap(), &refs).unwrap();
            assert!(record.target == t && record.request_id == "original-request");
            let job = Job::reopen(&record.job, deadline()).unwrap();
            assert!(
                job.members(deadline())
                    .unwrap()
                    .contains(&std::process::id())
            );
            drop(job);
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            atomic_json(&root.join("ready.json"), &listener.local_addr().unwrap()).unwrap();
            let mut sockets = Vec::new();
            let mut received = 0usize;
            let until = Instant::now() + Duration::from_secs(15);
            while !root.join("release").exists() && Instant::now() < until {
                while let Ok((socket, _)) = listener.accept() {
                    socket.set_nonblocking(true).unwrap();
                    sockets.push(socket);
                }
                use std::io::Read;
                for socket in &mut sockets {
                    match socket.read(&mut [0; 64]) {
                        Ok(n) => received += n,
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::WouldBlock
                                    | std::io::ErrorKind::ConnectionReset
                                    | std::io::ErrorKind::ConnectionAborted
                            ) => {}
                        Err(e) => panic!("original service read failed: {e}"),
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            atomic_json(&root.join("received.json"), &received).unwrap();
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        struct Release(PathBuf);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = std::fs::write(&self.0, b"original release");
            }
        }
        let release = Release(root.join("release"));
        let store = Store::new(root.join("state"));
        setup(&store);
        let refs = References::with_key(store.root().join("references"), [47; 32]);
        let mut environment = BTreeMap::from([(OsString::from(ROLE), root.as_os_str().to_owned())]);
        if let Some(system) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), system);
        }
        let executable = std::env::current_exe().unwrap();
        let arguments = ["--exact".into(), NAME.into(), "--test-threads=1".into()];
        let spec = SpawnSpec {
            program: &executable,
            arguments: &arguments,
            environment: &environment,
            directory: root,
        };
        let Begin::Fresh(intent) =
            start_intent::begin(&store, &refs, "original-request", &target(), deadline()).unwrap()
        else {
            panic!("fresh")
        };
        let mut running = None;
        intent
            .dispatch(|tx| {
                let value =
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())?;
                let record = load(&path(&store, "original-instance", "original-run")?, &refs)?;
                assert!(
                    record.outcome == Outcome::Spawned
                        && record.root.as_ref() == Some(value.root().identity())
                );
                assert_eq!(
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())
                        .err()
                        .unwrap()
                        .code,
                    "RUN_JOB_CONFLICT"
                );
                running = Some(value);
                Ok(())
            })
            .unwrap();
        wait_ready(root);
        let running = running.unwrap();
        let recovered = reopen_root(&store, &refs, &target(), deadline()).unwrap();
        assert!(recovered.root().identity() == running.root().identity());
        drop(running);
        drop(recovered);
        let recovered = reopen_root(&store, &refs, &target(), deadline()).unwrap();
        let endpoint: std::net::SocketAddr = read_json(&root.join("ready.json"), 4096).unwrap();
        {
            let mut tx = store.transaction(deadline()).unwrap();
            tx.registry.instances[0].endpoint = format!("http://{endpoint}/");
            tx.persist().unwrap();
            assert_eq!(
                discover_service(&tx, &refs, &target(), recovered.job(), deadline())
                    .err()
                    .unwrap()
                    .code,
                "SERVICE_DISCOVERY_UNAVAILABLE"
            );
            let connect = || {
                let stream =
                    std::net::TcpStream::connect_timeout(&endpoint, Duration::from_secs(2))
                        .unwrap();
                recovered
                    .job()
                    .identify_connected(stream, recovered.root().identity().image(), deadline())
                    .unwrap()
            };
            let (bound, owner) = connect();
            bind_service(
                &tx,
                &refs,
                &target(),
                recovered.job(),
                bound,
                &owner,
                deadline(),
            )
            .unwrap();
            let pointer = path(&store, "original-instance", "original-run").unwrap();
            let original_pointer = std::fs::read(&pointer).unwrap();
            let (bound, owner) = connect();
            bind_service(
                &tx,
                &refs,
                &target(),
                recovered.job(),
                bound,
                &owner,
                deadline(),
            )
            .unwrap();
            assert_eq!(std::fs::read(&pointer).unwrap(), original_pointer);
            let service = reopen_service(
                &store,
                &refs,
                &tx.registry.instances[0],
                &tx.registry.installations[0],
                deadline(),
            )
            .unwrap();
            struct Signer;
            impl crate::credentials::Credentials for Signer {
                fn get(&self, key: &str) -> Result<String> {
                    assert_eq!(key, "reference-signing-key");
                    use base64::Engine;
                    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([47; 32]))
                }
                fn set(&self, _: &str, _: &str) -> Result<()> {
                    panic!("no credential writes")
                }
                fn delete(&self, _: &str) -> Result<()> {
                    panic!("no credential deletion")
                }
            }
            let context = crate::transport::Context::load(
                &store,
                &tx.registry,
                &Signer,
                &tx.registry.instances[0],
                deadline(),
            )
            .unwrap();
            let _client = context
                .client(
                    &tx.registry.instances[0],
                    "original-token",
                    &crate::config::Config::default(),
                )
                .unwrap();
            assert!(service.owner().identity() == recovered.root().identity());
            assert!(service.job().contains(service.owner(), deadline()).unwrap());
            let mut changed_installation = tx.registry.installations[0].clone();
            changed_installation.digest = "c".repeat(64);
            assert!(
                reopen_service(
                    &store,
                    &refs,
                    &tx.registry.instances[0],
                    &changed_installation,
                    deadline()
                )
                .is_err()
            );
            let mut changed_instance = tx.registry.instances[0].clone();
            changed_instance.endpoint = "http://127.0.0.1:1/".into();
            assert!(
                reopen_service(
                    &store,
                    &refs,
                    &changed_instance,
                    &tx.registry.installations[0],
                    deadline()
                )
                .is_err()
            );
            let digest =
                Sha256::digest(serde_json::to_vec(&("original-instance", "original-run")).unwrap());
            let retention = store
                .root()
                .join("retentions")
                .join(format!("{digest:x}.json"));
            let saved = store.root().join("original-saved-retention.json");
            assert!(retention.starts_with(root) && saved.starts_with(root));
            std::fs::rename(&retention, &saved).unwrap();
            let (bound, owner) = connect();
            assert!(
                bind_service(
                    &tx,
                    &refs,
                    &target(),
                    recovered.job(),
                    bound,
                    &owner,
                    deadline()
                )
                .is_err()
            );
            assert!(!retention.exists());
            std::fs::rename(&saved, &retention).unwrap();
            assert_eq!(std::fs::read(&pointer).unwrap(), original_pointer);
        }
        let pointer_path = path(&store, "original-instance", "original-run").unwrap();
        let before = std::fs::read(&pointer_path).unwrap();
        let wrong = References::with_key(store.root().join("references"), [48; 32]);
        assert!(reopen_root(&store, &wrong, &target(), deadline()).is_err());
        assert_eq!(std::fs::read(&pointer_path).unwrap(), before);
        drop(release);
        let until = deadline();
        while recovered.root().verify(until).is_ok() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(recovered.root().verify(deadline()).is_err());
        assert!(reopen_root(&store, &refs, &target(), deadline()).is_err());
        let registry = store.read().unwrap();
        assert!(
            reopen_service(
                &store,
                &refs,
                &registry.instances[0],
                &registry.installations[0],
                deadline()
            )
            .is_err()
        );
        assert_eq!(
            read_json::<usize>(&root.join("received.json"), 4096).unwrap(),
            0
        );
    }
    fn wait_ready(root: &Path) {
        let until = deadline();
        while !root.join("ready.json").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(root.join("ready.json").exists());
    }
    #[test]
    fn failed_dispatch_keeps_job_identity_and_never_replays_pending_or_unknown() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        setup(&store);
        let refs = References::with_key(store.root().join("references"), [49; 32]);
        let invalid = temp.path().join("original-invalid.exe");
        std::fs::write(&invalid, b"original non executable").unwrap();
        let environment = BTreeMap::new();
        let spec = SpawnSpec {
            program: &invalid,
            arguments: &[],
            environment: &environment,
            directory: temp.path(),
        };
        let Begin::Fresh(intent) =
            start_intent::begin(&store, &refs, "original-request", &target(), deadline()).unwrap()
        else {
            panic!("fresh")
        };
        let result = intent
            .dispatch(|tx| {
                let mut changed = target();
                changed.input_hash = "c".repeat(64);
                assert_eq!(
                    launch_once(tx, &refs, "original-request", &changed, &spec, deadline())
                        .err()
                        .unwrap()
                        .code,
                    "RUN_JOB_CONFLICT"
                );
                assert!(!path(&store, "original-instance", "original-run")?.exists());
                assert_eq!(
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())
                        .err()
                        .unwrap()
                        .code,
                    "START_OUTCOME_UNKNOWN"
                );
                let p = path(&store, "original-instance", "original-run")?;
                let mut record = load(&p, &refs)?;
                assert!(record.outcome == Outcome::Unknown && record.root.is_none());
                assert_eq!(
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())
                        .err()
                        .unwrap()
                        .code,
                    "RUN_JOB_CONFLICT"
                );
                record.outcome = Outcome::Pending;
                save(&p, &refs, &record)?;
                assert_eq!(
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())
                        .err()
                        .unwrap()
                        .code,
                    "RUN_JOB_CONFLICT"
                );
                std::fs::write(&p, b"original corrupt journal").unwrap();
                assert!(
                    launch_once(tx, &refs, "original-request", &target(), &spec, deadline())
                        .is_err()
                );
                assert_eq!(std::fs::read(&p).unwrap(), b"original corrupt journal");
                Err(unknown())
            })
            .unwrap();
        assert_eq!(result.outcome, start_intent::Outcome::Unknown);
        assert!(reopen_root(&store, &refs, &target(), deadline()).is_err());
    }
}

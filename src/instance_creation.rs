//! Durable, account-free instance reservation before runtime preparation.
use crate::ports::Lease;
use crate::{
    catalog::Release,
    compatibility::Witness,
    doctor::Probe,
    error::{AppError, Result},
    references::References,
    runtime_store::{self, Plan},
    startup::Inputs,
    state::{Installation, Instance, Store, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Reserved,
    Registered,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    installation: Installation,
    plan: Plan,
    phase: Phase,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

pub enum Creation {
    Fresh(Reservation),
    Existing { instance: Instance, plan: Plan },
}
pub struct Evidence<'a, P: Probe> {
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
    pub probe: &'a P,
}
/// Keeps three local ports reserved until the caller is ready to dispatch.
/// Dropping closes only these local sockets, never any QQ process or account.
pub struct Reservation {
    plan: Plan,
    _ports: Lease,
}
impl Reservation {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
}

/// Read a signed creation identity without reserving ports or mutating state.
/// Callers must still compare it with the current registered instance/run.
pub fn reserved_plan(store: &Store, refs: &References, request_id: &str) -> Result<Option<Plan>> {
    crate::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
    let path = store
        .root()
        .join("instance-creations")
        .join(format!("{:x}.json", Sha256::digest(request_id.as_bytes())));
    reject_links(&path)?;
    let pointer: Pointer = match crate::config::read_json(&path, 4096) {
        Ok(pointer) => pointer,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(conflict());
    }
    let record: Record = refs.get("n1", &pointer.reference)?;
    if record.schema_version != 1 || record.request_id != request_id {
        return Err(conflict());
    }
    record.plan.ports.validate()?;
    for key in [&record.plan.instance_id, &record.plan.run_id] {
        crate::domain::request_key(key).map_err(|_| conflict())?;
    }
    Ok(Some(record.plan))
}

impl Inputs {
    /// A stable explicit creation key reserves one instance/run. No credentials,
    /// runtime contents or processes are created; all accounts remain unpreset.
    pub fn reserve_instance<P: Probe>(
        &self,
        store: &Store,
        refs: &References,
        request_id: &str,
        evidence: &Evidence<'_, P>,
        deadline: Instant,
    ) -> Result<Creation> {
        let Evidence {
            release,
            witnesses,
            probe,
        } = evidence;
        crate::deadline::check(deadline)?;
        crate::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
        release.require_authorization()?;
        let mut tx = store.transaction(deadline)?;
        let installation =
            self.with_revalidated(&tx, release, witnesses, *probe, deadline, |_, fresh| {
                Ok(fresh.installation().clone())
            })?;
        let parent = store.root().join("instance-creations");
        let path = parent.join(format!("{:x}.json", Sha256::digest(request_id.as_bytes())));
        reject_links(&path)?;
        let mut record = match crate::config::read_json::<Pointer>(&path, 4096) {
            Ok(pointer) => {
                if pointer.schema_version != 1 {
                    return Err(conflict());
                }
                let record: Record = refs.get("n1", &pointer.reference)?;
                if record.schema_version != 1
                    || record.request_id != request_id
                    || record.installation != installation
                    || record.plan.version != release.version
                {
                    return Err(conflict());
                }
                record.plan.ports.validate()?;
                for key in [&record.plan.instance_id, &record.plan.run_id] {
                    crate::domain::request_key(key).map_err(|_| conflict())?;
                }
                record
            }
            Err(error) if error.code == "FILE_NOT_FOUND" => {
                let ports = Lease::dynamic(deadline)?;
                let plan = Plan {
                    instance_id: random_id("instance")?,
                    run_id: random_id("run")?,
                    version: release.version.into(),
                    ports: ports.ports(),
                };
                if tx
                    .registry
                    .instances
                    .iter()
                    .any(|i| i.id == plan.instance_id || i.run_id == plan.run_id)
                {
                    return Err(conflict());
                }
                let mut record = Record {
                    schema_version: 1,
                    request_id: request_id.into(),
                    installation,
                    plan,
                    phase: Phase::Reserved,
                };
                private_dir(&parent)?;
                save(&path, refs, &record)?; // Claim precedes registration and all preparation.
                tx.registry.instances.push(instance(&record)?);
                crate::deadline::check(deadline)?;
                tx.persist()?;
                record.phase = Phase::Registered;
                save(&path, refs, &record)?;
                return Ok(Creation::Fresh(Reservation {
                    plan: record.plan,
                    _ports: ports,
                }));
            }
            Err(error) => return Err(error),
        };
        if let Some(current) = tx
            .registry
            .instances
            .iter()
            .find(|i| i.id == record.plan.instance_id)
        {
            let expected = instance(&record)?;
            if current.run_id != expected.run_id
                || current.installation_id != expected.installation_id
                || current.endpoint != expected.endpoint
                || current.ws_endpoint != expected.ws_endpoint
                || current.credential_key != expected.credential_key
            {
                return Err(conflict());
            }
            if record.phase == Phase::Reserved {
                record.phase = Phase::Registered;
                save(&path, refs, &record)?;
            }
            return Ok(Creation::Existing {
                instance: current.clone(),
                plan: record.plan,
            });
        }
        if record.phase == Phase::Registered {
            return Err(AppError::new(
                "INSTANCE_REMOVED",
                "The instance reserved by this creation key is no longer registered; the key cannot create a replacement.",
                4,
            ));
        }
        let ports = Lease::acquire(record.plan.ports, deadline)?;
        tx.registry.instances.push(instance(&record)?);
        crate::deadline::check(deadline)?;
        tx.persist()?;
        record.phase = Phase::Registered;
        save(&path, refs, &record)?;
        Ok(Creation::Fresh(Reservation {
            plan: record.plan,
            _ports: ports,
        }))
    }
}
fn instance(record: &Record) -> Result<Instance> {
    Ok(Instance {
        id: record.plan.instance_id.clone(),
        run_id: record.plan.run_id.clone(),
        installation_id: record.installation.id.clone(),
        endpoint: format!("http://127.0.0.1:{}/", record.plan.ports.http),
        ws_endpoint: Some(format!("ws://127.0.0.1:{}/event", record.plan.ports.events)),
        credential_key: runtime_store::api_credential_key(
            &record.plan.instance_id,
            &record.plan.run_id,
        )?,
        account: None,
        state: "preparing".into(),
        process_id: None,
        process_started: None,
    })
}
fn save(path: &std::path::Path, refs: &References, record: &Record) -> Result<()> {
    reject_links(path)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference: refs.issue("n1", record)?,
        },
    )
}
fn random_id(prefix: &str) -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| AppError::storage())?;
    Ok(format!(
        "{prefix}-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
fn conflict() -> AppError {
    AppError::new(
        "INSTANCE_CREATION_CONFLICT",
        "Creation key, installation or reserved instance identity does not match; no replacement was created.",
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        client::SessionInfo,
        compatibility::{EvidenceKind, Key, Verdict},
        config::Config,
        platform::Host,
        qq::Application,
    };
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    struct Original(Application);
    impl Probe for Original {
        fn host(&self) -> Host {
            Host {
                os: "windows".into(),
                os_build: Some(10000),
                native_arch: "x86_64".into(),
                process_arch: "x86_64".into(),
            }
        }
        fn qq_paths(&self) -> Result<Vec<PathBuf>> {
            Ok(vec![self.0.executable.clone()])
        }
        fn inspect_qq(&self, _: &Path, _: Instant) -> Result<Application> {
            Ok(self.0.clone())
        }
        fn artifact(&self, _: &Store, _: &Installation, _: Instant) -> Result<()> {
            Ok(())
        }
        fn api(
            &self,
            _: &Store,
            _: &Config,
            _: &Instance,
            _: &str,
            _: Instant,
        ) -> Result<SessionInfo> {
            panic!("no account API during instance reservation")
        }
    }
    fn deadline() -> Instant {
        Instant::now() + std::time::Duration::from_secs(5)
    }
    fn setup(root: &Path) -> (Store, References, Release, Vec<Witness>, Original, Inputs) {
        let store = Store::new(root.join("state"));
        let refs = References::with_key(store.root().join("references"), [65; 32]);
        let mut release = crate::catalog::lookup("v4.18.38").unwrap();
        release.authorization_evidence = Some("original reservation fixture");
        let probe = Original(Application {
            id: "original-qq".into(),
            version: "9.9.99".into(),
            build: "60001".into(),
            architecture: "x86_64".into(),
            fingerprint: "1".repeat(64),
            executable_sha256: "2".repeat(64),
            major_sha256: "3".repeat(64),
            entry_sha256: "4".repeat(64),
            metadata_sha256: "5".repeat(64),
            executable: root.join("original/QQ.exe"),
        });
        let witnesses = vec![Witness {
            key: Key::from_installation(probe.host(), &release, &probe.0).unwrap(),
            verdict: Verdict::Supported,
            reference: "original reservation witness".into(),
            kind: EvidenceKind::FrameworkStartup,
        }];
        let mut tx = store.transaction(deadline()).unwrap();
        if tx.registry.installations.is_empty() {
            tx.registry.installations.push(Installation {
                id: "original-installation".into(),
                version: release.version.into(),
                digest: release.archive.sha256.into(),
                directory: root.join("original-installation"),
            });
            tx.commit().unwrap();
        } else {
            assert_eq!(tx.registry.installations.len(), 1);
        }
        let inputs = crate::startup::inspect(
            &store,
            &store.read().unwrap(),
            &release,
            &witnesses,
            &probe,
            deadline(),
        )
        .unwrap();
        (store, refs, release, witnesses, probe, inputs)
    }
    fn claim_path(store: &Store) -> PathBuf {
        std::fs::read_dir(store.root().join("instance-creations"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    }
    #[test]
    fn stable_creation_key_reserves_ports_and_never_recreates_a_removed_instance() {
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, release, witnesses, probe, inputs) = setup(temp.path());
        let evidence = Evidence {
            release: &release,
            witnesses: &witnesses,
            probe: &probe,
        };
        assert!(
            reserved_plan(&store, &refs, "original-request")
                .unwrap()
                .is_none()
        );
        let Creation::Fresh(reservation) = inputs
            .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
            .unwrap()
        else {
            panic!("fresh reservation")
        };
        let plan = reservation.plan().clone();
        assert!(
            reserved_plan(&store, &refs, "original-request")
                .unwrap()
                .unwrap()
                == plan
        );
        assert!(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, plan.ports.http)).is_err());
        assert!(store.read().unwrap().instances[0].account.is_none());
        assert!(!store.root().join("runtimes").exists());
        assert!(!store.root().join("start-requests").exists());
        let Creation::Existing {
            instance,
            plan: same,
        } = inputs
            .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
            .unwrap()
        else {
            panic!("stable existing reservation")
        };
        assert!(same == plan);
        assert_eq!(instance.id, plan.instance_id);
        assert_eq!(store.read().unwrap().instances.len(), 1);
        drop(reservation);
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances.clear();
        tx.commit().unwrap();
        assert_eq!(
            inputs
                .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
                .err()
                .unwrap()
                .code,
            "INSTANCE_REMOVED"
        );
        assert!(store.read().unwrap().instances.is_empty());
    }
    #[test]
    fn automatic_preparation_requires_the_exact_signed_instance_and_run() {
        use crate::default_start::{self, Selection};
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, release, witnesses, probe, inputs) = setup(temp.path());
        let Selection::Create { request_id } =
            default_start::select(&store, &refs, release.version, deadline()).unwrap()
        else {
            panic!("creation")
        };
        let evidence = Evidence {
            release: &release,
            witnesses: &witnesses,
            probe: &probe,
        };
        let reserved = inputs
            .reserve_instance(&store, &refs, &request_id, &evidence, deadline())
            .unwrap();
        drop(reserved);
        let Selection::Create { request_id: same } =
            default_start::select(&store, &refs, release.version, deadline()).unwrap()
        else {
            panic!("resume")
        };
        assert_eq!(same, request_id);
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances[0].run_id = "other-original-run".into();
        tx.commit().unwrap();
        assert_eq!(
            default_start::select(&store, &refs, release.version, deadline())
                .err()
                .unwrap()
                .code,
            "AUTOMATIC_START_CONFLICT"
        );
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances.clear();
        tx.commit().unwrap();
        let Selection::Create { request_id: same } =
            default_start::select(&store, &refs, release.version, deadline()).unwrap()
        else {
            panic!("original checkpoint")
        };
        assert_eq!(same, request_id);
        assert_eq!(
            inputs
                .reserve_instance(&store, &refs, &same, &evidence, deadline())
                .err()
                .unwrap()
                .code,
            "INSTANCE_REMOVED"
        );
        assert!(store.read().unwrap().instances.is_empty());
    }
    #[test]
    fn interrupted_claim_reuses_exact_ids_ports_and_refuses_busy_or_corrupt_material() {
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, release, witnesses, probe, inputs) = setup(temp.path());
        let evidence = Evidence {
            release: &release,
            witnesses: &witnesses,
            probe: &probe,
        };
        let Creation::Fresh(reservation) = inputs
            .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
            .unwrap()
        else {
            panic!("fresh")
        };
        let plan = reservation.plan().clone();
        let path = claim_path(&store);
        let pointer: Pointer = crate::config::read_json(&path, 4096).unwrap();
        let mut record: Record = refs.get("n1", &pointer.reference).unwrap();
        record.phase = Phase::Reserved;
        save(&path, &refs, &record).unwrap();
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances.clear();
        tx.commit().unwrap();
        // Model the crash after the reserved claim, before registry publication.
        assert_eq!(
            inputs
                .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
                .err()
                .unwrap()
                .code,
            "ENDPOINT_IN_USE"
        );
        assert!(store.read().unwrap().instances.is_empty());
        drop(reservation);
        let Creation::Fresh(recovered) = inputs
            .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
            .unwrap()
        else {
            panic!("recovered exact reservation")
        };
        assert!(recovered.plan() == &plan);
        std::fs::write(&path, b"original corrupt claim").unwrap();
        assert!(
            inputs
                .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
                .is_err()
        );
        assert_eq!(store.read().unwrap().instances.len(), 1);
        assert_eq!(std::fs::read(path).unwrap(), b"original corrupt claim");
    }
    #[test]
    fn concurrent_same_key_creates_one_instance_and_one_fresh_reservation() {
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, release, witnesses, probe, inputs) = setup(temp.path());
        let evidence = Evidence {
            release: &release,
            witnesses: &witnesses,
            probe: &probe,
        };
        let results = std::thread::scope(|scope| {
            (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        match inputs
                            .reserve_instance(
                                &store,
                                &refs,
                                "original-concurrent",
                                &evidence,
                                deadline(),
                            )
                            .unwrap()
                        {
                            Creation::Fresh(reservation) => {
                                (reservation.plan().instance_id.clone(), true)
                            }
                            Creation::Existing { instance, .. } => (instance.id, false),
                        }
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(results.iter().filter(|(_, fresh)| *fresh).count(), 1);
        assert!(results.iter().all(|(id, _)| id == &results[0].0));
        assert_eq!(store.read().unwrap().instances.len(), 1);
    }
    #[test]
    fn independent_processes_share_one_creation_claim_and_run() {
        const ROOT: &str = "QQ_CLI_ORIGINAL_CREATION_PROCESS_ROOT";
        if let Some(root) = std::env::var_os(ROOT) {
            let root = PathBuf::from(root).canonicalize().unwrap();
            assert!(root.starts_with(std::env::temp_dir().canonicalize().unwrap()));
            assert_eq!(
                std::fs::read(root.join("original-process-marker")).unwrap(),
                b"original process test"
            );
            let pid = std::process::id();
            std::fs::write(root.join(format!("ready-{pid}")), b"original ready").unwrap();
            let end = Instant::now() + std::time::Duration::from_secs(15);
            while !root.join("original-go").exists() {
                assert!(Instant::now() < end);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let (store, refs, release, witnesses, probe, inputs) = setup(&root);
            let evidence = Evidence {
                release: &release,
                witnesses: &witnesses,
                probe: &probe,
            };
            let (plan, fresh) = match inputs
                .reserve_instance(
                    &store,
                    &refs,
                    "original-multiprocess",
                    &evidence,
                    deadline(),
                )
                .unwrap()
            {
                Creation::Fresh(reservation) => (reservation.plan().clone(), true),
                Creation::Existing { plan, .. } => (plan, false),
            };
            atomic_json(
                &root.join(format!("result-{pid}.json")),
                &serde_json::json!({"instance":plan.instance_id,"run":plan.run_id,"fresh":fresh}),
            )
            .unwrap();
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let (store, _, _, _, _, _) = setup(&root);
        std::fs::write(
            root.join("original-process-marker"),
            b"original process test",
        )
        .unwrap();
        let executable = std::env::current_exe().unwrap();
        let mut children = (0..4).map(|_| {
            let mut command = std::process::Command::new(&executable);
            command.args(["--exact","instance_creation::tests::independent_processes_share_one_creation_claim_and_run","--test-threads=1"])
                .env(ROOT,&root).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
            #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
            command.spawn().unwrap()
        }).collect::<Vec<_>>();
        let end = Instant::now() + std::time::Duration::from_secs(30);
        while children
            .iter()
            .any(|child| !root.join(format!("ready-{}", child.id())).exists())
        {
            assert!(
                Instant::now() < end,
                "original children did not reach barrier"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::write(root.join("original-go"), b"original release").unwrap();
        let pids = children.iter().map(|child| child.id()).collect::<Vec<_>>();
        loop {
            let completed = children
                .iter_mut()
                .map(|child| child.try_wait().unwrap().is_some())
                .collect::<Vec<_>>();
            if completed.iter().all(|done| *done) {
                break;
            }
            assert!(Instant::now() < end, "original children did not finish");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        for child in children {
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "original creation worker failed: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
        let results = pids
            .into_iter()
            .map(|pid| {
                crate::config::read_json::<serde_json::Value>(
                    &root.join(format!("result-{pid}.json")),
                    4096,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            results
                .iter()
                .filter(|result| result["fresh"] == true)
                .count(),
            1
        );
        assert!(
            results
                .iter()
                .all(|result| result["instance"] == results[0]["instance"]
                    && result["run"] == results[0]["run"])
        );
        assert_eq!(store.read().unwrap().instances.len(), 1);
        assert_eq!(
            std::fs::read_dir(store.root().join("instance-creations"))
                .unwrap()
                .count(),
            1
        );
        assert!(!store.root().join("runtimes").exists());
        assert!(!store.root().join("run-jobs").exists());
    }
}

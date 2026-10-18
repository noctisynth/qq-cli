//! Durable, account-free instance reservation before runtime preparation.
use crate::{
    catalog::Release,
    compatibility::Witness,
    doctor::Probe,
    error::{AppError, Result},
    references::References,
    runtime_config::Ports,
    runtime_store::{self, Plan},
    startup::Inputs,
    state::{Installation, Instance, Store, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::TcpListener;
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
    _ports: Vec<TcpListener>,
}
impl Reservation {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
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
                let ports = reserve_ports(None)?;
                let plan = Plan {
                    instance_id: random_id("instance")?,
                    run_id: random_id("run")?,
                    version: release.version.into(),
                    ports: addresses(&ports)?,
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
        let ports = reserve_ports(Some(record.plan.ports))?;
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
fn reserve_ports(fixed: Option<Ports>) -> Result<Vec<TcpListener>> {
    [fixed.map_or(0, |p|p.http),fixed.map_or(0, |p|p.events),fixed.map_or(0, |p|p.control)].into_iter().map(|port|
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,port)).map_err(|_| AppError::new("ENDPOINT_IN_USE", "A reserved local endpoint is unavailable; no alternate instance or port was selected.", 4))
    ).collect()
}
fn addresses(ports: &[TcpListener]) -> Result<Ports> {
    let port = |index: usize| {
        ports[index]
            .local_addr()
            .map(|a| a.port())
            .map_err(|_| AppError::storage())
    };
    let result = Ports {
        http: port(0)?,
        events: port(1)?,
        control: port(2)?,
    };
    result.validate()?;
    Ok(result)
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
        tx.registry.installations.push(Installation {
            id: "original-installation".into(),
            version: release.version.into(),
            digest: release.archive.sha256.into(),
            directory: root.join("original-installation"),
        });
        tx.commit().unwrap();
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
        let Creation::Fresh(reservation) = inputs
            .reserve_instance(&store, &refs, "original-request", &evidence, deadline())
            .unwrap()
        else {
            panic!("fresh reservation")
        };
        let plan = reservation.plan().clone();
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
}

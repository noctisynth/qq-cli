use super::*;
use crate::{
    napcat::compatibility::{EvidenceKind, Key, Verdict},
    napcat::qq::Application,
    native::platform::Host,
    storage::config::Config,
    transport::client::SessionInfo,
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
    fn api(&self, _: &Store, _: &Config, _: &Instance, _: &str, _: Instant) -> Result<SessionInfo> {
        panic!("no account API during instance reservation")
    }
}
fn deadline() -> Instant {
    Instant::now() + std::time::Duration::from_secs(5)
}
fn setup(root: &Path) -> (Store, References, Release, Vec<Witness>, Original, Inputs) {
    let store = Store::new(root.join("state"));
    let refs = References::with_key(store.root().join("references"), [65; 32]);
    let mut release = crate::napcat::install::catalog::lookup("v4.18.38").unwrap();
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
    let inputs = crate::napcat::runtime::startup::inspect(
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
    use crate::napcat::runtime::default_start::{self, Selection};
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
fn message_request_key_blocks_creation_before_any_reservation_is_published() {
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, release, witnesses, probe, inputs) = setup(temp.path());
    let scope = crate::core::domain::Scope {
        instance_id: "original-instance".into(),
        account: "10001".into(),
    };
    let crate::requests::ledger::Begin::Fresh(intent) = crate::requests::ledger::begin(
        &store,
        "shared-original-key",
        "send",
        &scope,
        &serde_json::json!({}),
        deadline(),
    )
    .unwrap() else {
        panic!("message intent")
    };
    intent
        .finish(crate::requests::ledger::Outcome::Unknown, None)
        .unwrap();
    let evidence = Evidence {
        release: &release,
        witnesses: &witnesses,
        probe: &probe,
    };
    assert_eq!(
        inputs
            .reserve_instance(&store, &refs, "shared-original-key", &evidence, deadline())
            .err()
            .unwrap()
            .code,
        "REQUEST_CONFLICT"
    );
    assert!(store.read().unwrap().instances.is_empty());
    assert!(!store.root().join("instance-creations").exists());
    assert!(!store.root().join("runtimes").exists());
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
    let pointer: Pointer = crate::storage::config::read_json(&path, 4096).unwrap();
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
            command.args(["--exact","napcat::runtime::instance_creation::tests::independent_processes_share_one_creation_claim_and_run","--test-threads=1"])
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
            crate::storage::config::read_json::<serde_json::Value>(
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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn macos_claims_reuse_protocol_without_accepting_legacy_authority() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (store, refs, release, _, _, _) = setup(&root);
    let call = |kind, key| {
        let tx = store.transaction(deadline()).unwrap();
        let installation = tx.registry.installations[0].clone();
        reserve_in(
            tx,
            Admission {
                refs: &refs,
                request_id: key,
                release: &release,
                installation,
                kind,
                guard: &|| Ok(()),
            },
            deadline(),
        )
    };
    let Creation::Fresh(reservation) = call("n2", "mac-request").unwrap() else {
        panic!("fresh")
    };
    let plan = reservation.plan().clone();
    assert!(
        macos_checkpoint(&store, &refs, "mac-request")
            .unwrap()
            .unwrap()
            .plan
            == plan
    );
    assert!(reserved_plan(&store, &refs, "mac-request").is_err());
    assert!(call("n1", "mac-request").is_err());
    assert!(matches!(
        call("n2", "mac-request").unwrap(),
        Creation::Existing { .. }
    ));
    assert!(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, plan.ports.http)).is_err());
    assert_eq!(store.read().unwrap().instances.len(), 1);
    drop(reservation);
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances.clear();
    tx.commit().unwrap();
    assert_eq!(
        call("n2", "mac-request").err().unwrap().code,
        "INSTANCE_REMOVED"
    );
    let legacy = call("n1", "legacy-request").unwrap();
    drop(legacy);
    assert!(macos_checkpoint(&store, &refs, "legacy-request").is_err());
    assert!(call("n2", "legacy-request").is_err());
}
#[test]
fn lost_creation_guard_after_claim_cannot_register_or_select_another_run() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let (store, refs, release, _, _, _) = setup(&root);
    let tx = store.transaction(deadline()).unwrap();
    let installation = tx.registry.installations[0].clone();
    let guard = || {
        if path(&store, "guarded-request")?.exists() {
            return Err(conflict());
        }
        Ok(())
    };
    assert!(
        reserve_in(
            tx,
            Admission {
                refs: &refs,
                request_id: "guarded-request",
                release: &release,
                installation: installation.clone(),
                kind: "n1",
                guard: &guard
            },
            deadline()
        )
        .is_err()
    );
    assert!(store.read().unwrap().instances.is_empty());
    let original = reserved_plan(&store, &refs, "guarded-request")
        .unwrap()
        .unwrap();
    let tx = store.transaction(deadline()).unwrap();
    let Creation::Fresh(reservation) = reserve_in(
        tx,
        Admission {
            refs: &refs,
            request_id: "guarded-request",
            release: &release,
            installation,
            kind: "n1",
            guard: &|| Ok(()),
        },
        deadline(),
    )
    .unwrap() else {
        panic!("fresh")
    };
    assert!(reservation.plan() == &original);
}

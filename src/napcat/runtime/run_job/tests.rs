use super::journal::{load, path, save};
use super::*;
use crate::napcat::runtime::start_intent::{self, Begin};
use crate::native::job::SpawnSpec;
use crate::storage::{
    config::read_json,
    references::References,
    state::{Installation, Instance, Store, atomic_json},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::{Duration, Instant};
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
    const NAME: &str = "napcat::runtime::run_job::tests::signed_job_precedes_original_child_and_root_recovers_without_respawn";
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
            let value = launch_once(tx, &refs, "original-request", &target(), &spec, deadline())?;
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
                std::net::TcpStream::connect_timeout(&endpoint, Duration::from_secs(2)).unwrap();
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
        let mut inconsistent = load(&pointer, &refs).unwrap();
        let foreign = Pinned::observe(std::process::id(), deadline()).unwrap();
        inconsistent.prelogin = Some(Service {
            owner: foreign.identity().clone(),
            endpoint: "http://127.0.0.1:19003/".into(),
        });
        save(&pointer, &refs, &inconsistent).unwrap();
        assert_eq!(
            reopen_service(
                &store,
                &refs,
                &tx.registry.instances[0],
                &tx.registry.installations[0],
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "RUN_JOB_CONFLICT"
        );
        std::fs::write(&pointer, &original_pointer).unwrap();
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
        impl crate::storage::credentials::Credentials for Signer {
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
        let context = crate::app::backend::Context::load(
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
                &crate::storage::config::Config::default(),
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
                launch_once(tx, &refs, "original-request", &target(), &spec, deadline()).is_err()
            );
            assert_eq!(std::fs::read(&p).unwrap(), b"original corrupt journal");
            Err(unknown())
        })
        .unwrap();
    assert_eq!(result.outcome, start_intent::Outcome::Unknown);
    assert!(reopen_root(&store, &refs, &target(), deadline()).is_err());
}

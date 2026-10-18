use super::*;
use crate::storage::state::{atomic_json, private_dir};
use sha2::{Digest, Sha256};
use std::time::Duration;
fn end() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn fixture() -> (tempfile::TempDir, Store, References, Instance, Installation) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [93; 32]);
    let instance = Instance {
        id: "test-instance".into(),
        run_id: "test-run".into(),
        installation_id: "test-installation".into(),
        endpoint: "http://127.0.0.1:19001/".into(),
        ws_endpoint: None,
        credential_key: "old-credential".into(),
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    };
    let installation = Installation {
        id: instance.installation_id.clone(),
        version: "test".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("installation"),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.push(instance.clone());
    tx.registry.installations.push(installation.clone());
    tx.commit().unwrap();
    super::super::stop::execute(&store, &refs, &instance, end()).unwrap();
    let instance = store.read().unwrap().instances.remove(0);
    (temp, store, refs, instance, installation)
}
fn run(
    store: &Store,
    refs: &References,
    source: &Instance,
    installation: &Installation,
) -> Result<NextRun> {
    let mut tx = store.transaction(end())?;
    reserve(&mut tx, refs, source, installation, "test", end())
}
fn plan(next: NextRun) -> (Plan, String) {
    match next.creation {
        Creation::Fresh(reserved) => (reserved.plan().clone(), next.request_id),
        Creation::Existing { plan, .. } => (plan, next.request_id),
    }
}
#[test]
fn stable_instance_gets_one_fresh_account_free_run_and_keeps_old_material() {
    let (_temp, store, refs, mut source, installation) = fixture();
    source.account = Some("10001".into());
    source.process_id = Some(123);
    source.process_started = Some("original-observation".into());
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = source.clone();
    tx.commit().unwrap();
    let marker = store.root().join("original-write.json");
    std::fs::write(&marker, b"preserve unresolved write").unwrap();
    let (first, key) = plan(run(&store, &refs, &source, &installation).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    assert_eq!(current.id, source.id);
    assert_ne!(current.run_id, source.run_id);
    assert_eq!(current.state, "preparing");
    assert_eq!(current.account, None);
    assert_eq!(current.process_id, None);
    assert_eq!(current.process_started, None);
    assert_ne!(current.credential_key, source.credential_key);
    let (again, again_key) = plan(run(&store, &refs, &source, &installation).unwrap());
    assert!(first == again);
    assert_eq!(key, again_key);
    assert!(active(&store, &refs, &current, end()).unwrap().unwrap().0 == first);
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.registry.instances[0].account = Some("10002".into());
    tx.commit().unwrap();
    let expected = store.read().unwrap().instances.remove(0);
    let next = run(&store, &refs, &source, &installation).unwrap();
    let Creation::Existing { instance, .. } = next.creation else {
        panic!("existing run expected")
    };
    assert_eq!(instance, expected);
    assert_eq!(store.read().unwrap().instances[0], expected);
    assert_eq!(std::fs::read(marker).unwrap(), b"preserve unresolved write");
    assert!(!store.root().join("runtimes").exists());
    assert!(!store.root().join("run-jobs").exists());
}
#[test]
fn failed_registry_publication_reuses_the_signed_plan_after_unlock() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temp, store, refs, source, installation) = fixture();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(store.root().join("registry.json"))
        .unwrap();
    assert!(run(&store, &refs, &source, &installation).is_err());
    assert_eq!(store.read().unwrap().instances[0], source);
    let saved = journal::load(
        &store,
        &refs,
        false,
        &source_key(&source.id, &source.run_id).unwrap(),
    )
    .unwrap()
    .unwrap();
    drop(held);
    let (resumed, _) = plan(run(&store, &refs, &source, &installation).unwrap());
    assert!(resumed == saved.plan);
    assert_eq!(
        std::fs::read_dir(store.root().join("run-transitions"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        std::fs::read_dir(store.root().join("run-origins"))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn missing_origin_or_removed_registration_is_not_repaired_or_recreated() {
    let (_temp, store, refs, source, installation) = fixture();
    let (_, request) = plan(run(&store, &refs, &source, &installation).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    let path = store
        .root()
        .join("run-origins")
        .join(format!("{:x}.json", Sha256::digest(request.as_bytes())));
    std::fs::remove_file(&path).unwrap();
    assert!(active(&store, &refs, &current, end()).is_err());
    assert!(!path.exists());
    assert!(run(&store, &refs, &source, &installation).is_err());
    assert!(!path.exists());
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.clear();
    tx.commit().unwrap();
    assert!(run(&store, &refs, &source, &installation).is_err());
    assert!(store.read().unwrap().instances.is_empty());
}
#[test]
fn message_key_occupancy_refuses_transition_without_replacing_the_plan() {
    let (_temp, store, refs, source, installation) = fixture();
    let ports = Lease::dynamic(end()).unwrap();
    let record = Record {
        schema_version: 1,
        source: source.clone(),
        installation: installation.clone(),
        plan: Plan {
            instance_id: source.id.clone(),
            run_id: "reserved-run".into(),
            version: "test".into(),
            ports: ports.ports(),
        },
        root_run_id: source.run_id.clone(),
    };
    drop(ports);
    journal::save(
        &store,
        &refs,
        false,
        &source_key(&source.id, &source.run_id).unwrap(),
        &record,
    )
    .unwrap();
    let request = key(&record.plan.instance_id, &record.plan.run_id).unwrap();
    private_dir(&store.root().join("requests")).unwrap();
    let marker = store
        .root()
        .join("requests")
        .join(format!("{:x}.json", Sha256::digest(request.as_bytes())));
    std::fs::write(&marker, b"original occupied request").unwrap();
    assert_eq!(
        run(&store, &refs, &source, &installation)
            .err()
            .unwrap()
            .code,
        "REQUEST_CONFLICT"
    );
    assert_eq!(store.read().unwrap().instances[0], source);
    assert!(!store.root().join("run-origins").exists());
}
#[test]
fn successor_runs_keep_the_authenticated_root_for_default_retirement() {
    use serde_json::json;
    let (_temp, store, refs, source, installation) = fixture();
    let directory = store.root().join("instance-creations");
    private_dir(&directory).unwrap();
    let reference=refs.issue("n1",&json!({"schemaVersion":1,"requestId":"original-default","installation":installation,"plan":{"instanceId":source.id,"runId":source.run_id,"version":"test","ports":{"http":19001,"events":19002,"control":19003}},"phase":"registered"})).unwrap();
    atomic_json(
        &directory.join(format!("{:x}.json", Sha256::digest(b"original-default"))),
        &json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    private_dir(&store.root().join("automatic-start")).unwrap();
    let reference = refs
        .issue(
            "d1",
            &json!({"schemaVersion":1,"requestId":"original-default","version":"test"}),
        )
        .unwrap();
    atomic_json(
        &store.root().join("automatic-start/active.json"),
        &json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    let mut previous = source.clone();
    for _ in 0..2 {
        let (_, request) = plan(run(&store, &refs, &previous, &installation).unwrap());
        assert_eq!(
            crate::requests::keys::guard_message(&store, &request)
                .err()
                .unwrap()
                .code,
            "REQUEST_CONFLICT"
        );
        let current = store.read().unwrap().instances.remove(0);
        super::super::stop::execute(&store, &refs, &current, end()).unwrap();
        previous = store.read().unwrap().instances.remove(0);
        assert!(descends_from(&store, &refs, &previous, &source.run_id).unwrap());
    }
    super::super::remove::execute(&store, &refs, &source.id, end()).unwrap();
    assert!(!store.root().join("automatic-start/active.json").exists());
    assert_eq!(
        std::fs::read_dir(store.root().join("instance-creations"))
            .unwrap()
            .count(),
        3
    );
}

#[test]
fn absent_or_wrong_stop_authority_does_not_claim_a_new_generation() {
    for missing in [true, false] {
        let (_temp, store, refs, source, installation) = fixture();
        let wrong = References::with_key(store.root().join("references"), [94; 32]);
        if missing {
            let path = std::fs::read_dir(store.root().join("instance-stops"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            std::fs::remove_file(path).unwrap();
        }
        let authority = if missing { &refs } else { &wrong };
        assert!(run(&store, authority, &source, &installation).is_err());
        assert_eq!(store.read().unwrap().instances[0], source);
        assert!(!store.root().join("run-transitions").exists());
        assert!(!store.root().join("run-origins").exists());
    }
}

#[test]
fn a_registered_new_run_is_not_recreated_after_registry_rollback() {
    let (_temp, store, refs, source, installation) = fixture();
    let (_, request) = plan(run(&store, &refs, &source, &installation).unwrap());
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = source.clone();
    tx.commit().unwrap();
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    assert!(run(&store, &refs, &source, &installation).is_err());
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    assert!(has_origin_for_request(&store, &request).unwrap());
}

#[test]
fn internal_generation_start_key_uses_existing_offline_request_lookup() {
    use base64::Engine;
    struct SigningOnly;
    impl crate::storage::credentials::Credentials for SigningOnly {
        fn get(&self, key: &str) -> Result<String> {
            if key == "reference-signing-key" {
                Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([93; 32]))
            } else {
                Err(AppError::storage())
            }
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            panic!("request lookup must not provision credentials")
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("request lookup must not delete credentials")
        }
    }
    let (_temp, store, refs, source, installation) = fixture();
    let (_, request) = plan(run(&store, &refs, &source, &installation).unwrap());
    let value = serde_json::to_value(
        crate::requests::query::get(&store, &request, None, &SigningOnly, end()).unwrap(),
    )
    .unwrap();
    assert_eq!(value["data"]["requestId"], request);
    assert_eq!(value["data"]["instanceId"], source.id);
    assert_eq!(value["data"]["operation"], "napcat.start");
    assert_eq!(value["data"]["identityVerified"], false);
    assert!(value.get("meta").is_none());
    assert!(!store.root().join("runtimes").exists());
}

#[test]
fn lost_known_origin_of_a_stopped_successor_does_not_reset_its_root() {
    let (_temp, store, refs, source, installation) = fixture();
    let (_, request) = plan(run(&store, &refs, &source, &installation).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    super::super::stop::execute(&store, &refs, &current, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    let path = store
        .root()
        .join("run-origins")
        .join(format!("{:x}.json", Sha256::digest(request.as_bytes())));
    std::fs::remove_file(&path).unwrap();
    assert!(run(&store, &refs, &stopped, &installation).is_err());
    assert!(descends_from(&store, &refs, &stopped, &source.run_id).is_err());
    assert_eq!(store.read().unwrap().instances[0], stopped);
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_dir(store.root().join("run-transitions"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn concurrent_metadata_requests_share_one_generation_and_internal_start_key() {
    let (_temp, store, refs, source, installation) = fixture();
    let results = std::thread::scope(|scope| {
        let workers = (0..4)
            .map(|_| scope.spawn(|| plan(run(&store, &refs, &source, &installation).unwrap())))
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    for result in &results {
        assert!(result.0 == results[0].0);
        assert_eq!(result.1, results[0].1);
    }
    assert_eq!(store.read().unwrap().instances.len(), 1);
    assert_eq!(
        std::fs::read_dir(store.root().join("run-transitions"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        std::fs::read_dir(store.root().join("run-origins"))
            .unwrap()
            .count(),
        1
    );
}

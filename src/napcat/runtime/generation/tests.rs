use super::*;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::napcat::runtime::macos::stop as platform_stop;
#[cfg(windows)]
use crate::napcat::runtime::stop as platform_stop;
use crate::storage::state::{atomic_json, private_dir};
use sha2::{Digest, Sha256};
use std::time::Duration;
fn end() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn fixture() -> (tempfile::TempDir, Store, References, Instance, Installation) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
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
        directory: temp.path().canonicalize().unwrap().join("installation"),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.push(instance.clone());
    tx.registry.installations.push(installation.clone());
    tx.commit().unwrap();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    let instance = store.read().unwrap().instances.remove(0);
    (temp, store, refs, instance, installation)
}
fn run(
    store: &Store,
    refs: &References,
    source: &Instance,
    installation: &Installation,
) -> Result<NextRun> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let lifecycle = crate::napcat::runtime::lifecycle_lock(store, end())?;
    let mut tx = store.transaction(end())?;
    #[cfg(windows)]
    {
        reserve(&mut tx, refs, source, installation, "test", end())
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        reserve_macos(&mut tx, refs, source, installation, &lifecycle, end())
    }
}
fn plan(next: NextRun) -> (Plan, String) {
    match next.creation {
        Creation::Fresh(reserved) => (reserved.plan().clone(), next.request_id),
        Creation::Existing { plan, .. } => (plan, next.request_id),
    }
}
#[cfg(windows)]
#[test]
fn claimed_version_target_remains_protected_when_registry_publication_is_interrupted() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temp, store, refs, source, original) = fixture();
    let target = add_version(&store, &original);
    super::super::version_intent::begin(
        &store,
        &refs,
        &source,
        &target,
        super::super::version_intent::Operation::Update,
        end(),
    )
    .unwrap();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(store.root().join("registry.json"))
        .unwrap();
    assert!(switch(&store, &refs, &source, &target).is_err());
    assert_eq!(store.read().unwrap().instances[0], source);
    assert_eq!(store.read().unwrap().maintenance_intents.len(), 1);
    let record = journal::load(
        &store,
        &refs,
        false,
        &source_key(&source.id, &source.run_id).unwrap(),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        crate::napcat::install::installation::uninstall(&store, &target.id, &refs, end())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
    assert!(!store.root().join("installation-operations").exists());
    drop(held);
    let (resumed, _) = plan(switch(&store, &refs, &source, &target).unwrap());
    assert!(resumed == record.plan);
    assert!(store.read().unwrap().maintenance_intents.is_empty());
    let current = store.read().unwrap().instances.remove(0);
    assert_eq!(
        maintenance_operation(&store, &refs, &current).unwrap(),
        Some(super::super::version_intent::Operation::Update)
    );
}

#[cfg(windows)]
#[test]
fn version_change_uses_the_source_stop_proof_and_keeps_both_installations() {
    let (_temp, store, refs, source, original) = fixture();
    let target = add_version(&store, &original);
    let before = store.read().unwrap();
    assert!(run(&store, &refs, &source, &target).is_err());
    assert_eq!(store.read().unwrap().instances, before.instances);
    let (first, request) = plan(switch(&store, &refs, &source, &target).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    assert_eq!(current.id, source.id);
    assert_ne!(current.run_id, source.run_id);
    assert_eq!(current.installation_id, target.id);
    assert_eq!(first.version, target.version);
    assert!(current.account.is_none() && current.process_id.is_none());
    assert_eq!(store.read().unwrap().installations, before.installations);
    assert_eq!(
        rollback_installation(&store, &refs, &current).unwrap(),
        Some(original)
    );
    let (again, key) = plan(switch(&store, &refs, &source, &target).unwrap());
    assert!(again == first);
    assert_eq!(key, request);
    assert!(active(&store, &refs, &current, end()).unwrap().is_some());
    assert!(!store.root().join("runtimes").exists());
    assert!(!store.root().join("run-jobs").exists());
}

#[cfg(windows)]
#[test]
fn rollback_reference_survives_restart_and_changes_only_on_a_version_switch() {
    let (_temp, store, refs, source, original) = fixture();
    let target = add_version(&store, &original);
    drop(switch(&store, &refs, &source, &target).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    let error = crate::napcat::install::installation::uninstall(&store, &original.id, &refs, end())
        .err()
        .unwrap();
    assert_eq!(error.code, "INSTALLATION_IN_USE");
    assert_eq!(
        serde_json::to_value(error.details.unwrap()).unwrap()["instances"][0],
        current.id
    );
    platform_stop::execute(&store, &refs, &current, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    {
        let mut tx = store.transaction(end()).unwrap();
        drop(reserve(&mut tx, &refs, &stopped, &target, &target.version, end()).unwrap());
    }
    let restarted = store.read().unwrap().instances.remove(0);
    assert_eq!(
        rollback_installation(&store, &refs, &restarted).unwrap(),
        Some(original.clone())
    );
    assert_ne!(restarted.run_id, current.run_id);
    platform_stop::execute(&store, &refs, &restarted, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    drop(switch(&store, &refs, &stopped, &original).unwrap());
    let restored = store.read().unwrap().instances.remove(0);
    assert_eq!(restored.installation_id, original.id);
    assert_eq!(
        rollback_installation(&store, &refs, &restored).unwrap(),
        Some(target.clone())
    );
    assert_eq!(
        crate::napcat::install::installation::uninstall(&store, &target.id, &refs, end())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
}

#[cfg(windows)]
#[test]
fn changing_the_original_installation_cannot_borrow_the_target_versions_stop_proof() {
    let (_temp, store, refs, source, original) = fixture();
    let target = add_version(&store, &original);
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.installations[0].digest = "c".repeat(64);
    tx.commit().unwrap();
    let before = store.read().unwrap();
    assert!(switch(&store, &refs, &source, &target).is_err());
    assert_eq!(store.read().unwrap().instances, before.instances);
    assert!(!store.root().join("run-transitions").exists());
}

#[cfg(windows)]
#[test]
fn version_record_cannot_masquerade_as_legacy_or_pin_its_current_installation_as_rollback() {
    let (_temp, store, refs, source, original) = fixture();
    let target = add_version(&store, &original);
    drop(switch(&store, &refs, &source, &target).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    let key = key(&current.id, &current.run_id).unwrap();
    let mut record = journal::load(&store, &refs, true, &key).unwrap().unwrap();
    record.schema_version = 1;
    journal::save(&store, &refs, true, &key, &record).unwrap();
    assert!(rollback_installation(&store, &refs, &current).is_err());
    record.schema_version = 2;
    record.rollback_installation = Some(target);
    journal::save(&store, &refs, true, &key, &record).unwrap();
    assert!(rollback_installation(&store, &refs, &current).is_err());
}

#[cfg(windows)]
fn add_version(store: &Store, original: &Installation) -> Installation {
    let target = Installation {
        id: "target-installation".into(),
        version: "target".into(),
        digest: "b".repeat(64),
        directory: original.directory.with_file_name("target-installation"),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.installations.push(target.clone());
    tx.commit().unwrap();
    target
}
#[cfg(windows)]
fn switch(
    store: &Store,
    refs: &References,
    source: &Instance,
    target: &Installation,
) -> Result<NextRun> {
    let mut tx = store.transaction(end())?;
    reserve_to(&mut tx, refs, source, target, &target.version, end())
}

#[cfg(windows)]
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
#[cfg(windows)]
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
    assert!(has_history(&store, &current).unwrap());
    assert!(rollback_installation(&store, &refs, &current).is_err());
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
        source_installation: None,
        rollback_installation: None,
        maintenance_reference: None,
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
    let reference=refs.issue(if cfg!(windows) {"n1"} else {"n2"},&json!({"schemaVersion":1,"requestId":"original-default","installation":installation,"plan":{"instanceId":source.id,"runId":source.run_id,"version":"test","ports":{"http":19001,"events":19002,"control":19003}},"phase":"registered"})).unwrap();
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
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert!(matches!(
            super::super::default_start::select_macos(&store, &refs, "test", end()).unwrap(),
            super::super::default_start::Selection::Existing(selected) if selected == current
        ));
        platform_stop::execute(&store, &refs, &current, end()).unwrap();
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
            let path = std::fs::read_dir(store.root().join(if cfg!(windows) {
                "instance-stops"
            } else {
                "macos-stops"
            }))
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
    platform_stop::execute(&store, &refs, &current, end()).unwrap();
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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn windows_generation_reference_cannot_authorize_a_macos_successor() {
    use serde_json::json;
    let (_temp, store, refs, source, installation) = fixture();
    let (_, request) = plan(run(&store, &refs, &source, &installation).unwrap());
    let current = store.read().unwrap().instances.remove(0);
    let record = journal::load(&store, &refs, true, &request)
        .unwrap()
        .unwrap();
    let reference = refs.issue("g1", &record).unwrap();
    let path = store
        .root()
        .join("run-origins")
        .join(format!("{:x}.json", Sha256::digest(request.as_bytes())));
    atomic_json(&path, &json!({"schemaVersion":1,"reference":reference})).unwrap();
    assert!(active(&store, &refs, &current, end()).is_err());
    assert_eq!(store.read().unwrap().instances[0], current);
}

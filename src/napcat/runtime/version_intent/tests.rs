use super::*;
use std::time::Duration;
fn end() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
fn fixture() -> (tempfile::TempDir, Store, References, Instance, Installation) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [73; 32]);
    let source = Instance {
        id: "original-instance".into(),
        run_id: "original-run".into(),
        installation_id: "original-installation".into(),
        endpoint: "http://127.0.0.1:19001/".into(),
        ws_endpoint: None,
        credential_key: "original-key".into(),
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    };
    let original = Installation {
        id: source.installation_id.clone(),
        version: "v4.18.37".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("original"),
    };
    let target = Installation {
        id: "target-installation".into(),
        version: "v4.18.38".into(),
        digest: "b".repeat(64),
        directory: temp.path().join("target"),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.push(source.clone());
    tx.registry.installations.extend([original, target.clone()]);
    tx.commit().unwrap();
    (temp, store, refs, source, target)
}
#[test]
fn unknown_source_is_refused_before_any_signed_claim() {
    let (_temp, store, refs, mut source, target) = fixture();
    source.state = "unknown".into();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = source.clone();
    tx.commit().unwrap();
    assert!(begin(&store, &refs, &source, &target, Operation::Update, end()).is_err());
    assert!(!store.root().join("version-maintenance").exists());
    assert!(store.read().unwrap().maintenance_intents.is_empty());
}

#[test]
fn intent_precedes_stop_and_blocks_old_start_and_target_uninstall() {
    let (_temp, store, refs, source, target) = fixture();
    let first = begin(&store, &refs, &source, &target, Operation::Update, end()).unwrap();
    assert_eq!(store.read().unwrap().instances[0], source);
    assert!(!store.root().join("instance-stops").exists());
    let again = begin(&store, &refs, &source, &target, Operation::Update, end()).unwrap();
    assert_eq!(first.reference, again.reference);
    assert!(begin(&store, &refs, &source, &target, Operation::Rollback, end()).is_err());
    let error = guard_start(&store, &refs, &store.read().unwrap(), &source)
        .err()
        .unwrap();
    assert_eq!(error.code, "VERSION_MAINTENANCE_PENDING");
    assert_eq!(
        error.next_action.unwrap()["args"]["version"],
        target.version
    );
    assert_eq!(
        crate::napcat::install::installation::uninstall(&store, &target.id, &refs, end())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
    super::super::stop::execute(&store, &refs, &source, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    assert!(guard_start(&store, &refs, &store.read().unwrap(), &stopped).is_err());
    assert_eq!(
        begin(&store, &refs, &stopped, &target, Operation::Update, end())
            .unwrap()
            .reference,
        first.reference
    );
    super::super::remove::execute(&store, &refs, &source.id, end()).unwrap();
    let removed = store.read().unwrap();
    assert!(removed.instances.is_empty() && removed.maintenance_intents.is_empty());
    assert!(
        journal::pointer(&store, &source.id, &source.run_id)
            .unwrap()
            .is_some()
    );
    assert!(begin(&store, &refs, &stopped, &target, Operation::Update, end()).is_err());
}
#[test]
fn missing_index_or_marker_after_stop_is_not_repaired() {
    let (_temp, store, refs, source, target) = fixture();
    let intent = begin(&store, &refs, &source, &target, Operation::Update, end()).unwrap();
    super::super::stop::execute(&store, &refs, &source, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    let index = std::fs::read_dir(store.root().join("version-maintenance"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = std::fs::read(&index).unwrap();
    std::fs::remove_file(&index).unwrap();
    assert!(guard_start(&store, &refs, &store.read().unwrap(), &stopped).is_err());
    assert!(begin(&store, &refs, &stopped, &target, Operation::Update, end()).is_err());
    std::fs::write(index, original).unwrap();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.maintenance_intents.clear();
    tx.commit().unwrap();
    assert!(begin(&store, &refs, &stopped, &target, Operation::Update, end()).is_err());
    assert!(store.read().unwrap().maintenance_intents.is_empty());
    assert_eq!(
        journal::load(&store, &refs, &intent.reference)
            .unwrap()
            .target(),
        &target
    );
}
#[test]
fn failed_marker_publication_recovers_only_the_unchanged_source() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temp, store, refs, source, target) = fixture();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(store.root().join("registry.json"))
        .unwrap();
    assert!(begin(&store, &refs, &source, &target, Operation::Update, end()).is_err());
    assert!(store.read().unwrap().maintenance_intents.is_empty());
    let reference = journal::pointer(&store, &source.id, &source.run_id)
        .unwrap()
        .unwrap();
    assert!(!store.root().join("instance-stops").exists());
    drop(held);
    assert_eq!(
        begin(&store, &refs, &source, &target, Operation::Update, end())
            .unwrap()
            .reference,
        reference
    );
}

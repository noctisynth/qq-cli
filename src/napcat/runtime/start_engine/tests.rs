use super::*;

#[test]
fn lifecycle_lock_blocks_uninstall_before_registry_or_directory_changes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [42; 32]);
    let end = Instant::now() + Duration::from_secs(3);
    let held = operation_lock(&store, end).unwrap();
    let error = crate::napcat::install::installation::uninstall(
        &store,
        "original-target",
        &refs,
        Instant::now() + Duration::from_millis(30),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "START_BUSY");
    assert!(!store.root().join("registry.json").exists());
    assert!(!store.root().join("installation-operations").exists());
    drop(held);
    let error =
        crate::napcat::install::installation::uninstall(&store, "original-target", &refs, end)
            .err()
            .unwrap();
    assert_eq!(error.code, "INSTALLATION_NOT_FOUND");
}

#[test]
fn startup_lock_serializes_creation_without_blocking_state_observation() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let end = Instant::now() + Duration::from_secs(3);
    let held = operation_lock(&store, end).unwrap();
    assert!(store.read().unwrap().instances.is_empty());
    let tx = store.transaction(end).unwrap();
    drop(tx);
    let error = operation_lock(&store, Instant::now() + Duration::from_millis(25))
        .err()
        .unwrap();
    assert_eq!(error.code, "START_BUSY");
    assert!(error.retry_safe);
    drop(held);
    operation_lock(&store, end).unwrap();
}

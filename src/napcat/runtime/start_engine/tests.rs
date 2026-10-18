use super::*;

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

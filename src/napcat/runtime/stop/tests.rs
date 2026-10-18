use super::*;
use crate::storage::state::Installation;

fn end() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
pub(super) fn fixture() -> (tempfile::TempDir, Store, References, Instance, Target) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [17; 32]);
    let instance = Instance {
        id: "test-instance".into(),
        installation_id: "test-installation".into(),
        endpoint: "http://127.0.0.1:19001".into(),
        ws_endpoint: None,
        credential_key: "test-credential".into(),
        account: None,
        run_id: "test-run".into(),
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    };
    let target = Target {
        instance_id: instance.id.clone(),
        run_id: instance.run_id.clone(),
        installation_id: instance.installation_id.clone(),
        installation_digest: "a".repeat(64),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.installations.push(Installation {
        id: target.installation_id.clone(),
        version: "test".into(),
        digest: target.installation_digest.clone(),
        directory: temp.path().join("installation"),
    });
    tx.registry.instances.push(instance.clone());
    tx.commit().unwrap();
    (temp, store, refs, instance, target)
}

#[test]
fn composed_stop_keeps_the_outer_lifecycle_lock_until_the_caller_releases_it() {
    let (_temp, store, refs, instance, _target) = fixture();
    let operation = crate::napcat::runtime::operation_lock::acquire(&store, end()).unwrap();
    let blocked = execute(
        &store,
        &refs,
        &instance,
        Instant::now() + Duration::from_millis(30),
    )
    .err()
    .unwrap();
    assert_eq!(blocked.code, "START_BUSY");
    assert_eq!(store.read().unwrap().instances[0], instance);
    assert!(!locked(&store, &refs, &instance, end()).unwrap().reused);
    let blocked = crate::napcat::runtime::operation_lock::acquire(
        &store,
        Instant::now() + Duration::from_millis(30),
    )
    .err()
    .unwrap();
    assert_eq!(blocked.code, "START_BUSY");
    drop(operation);
    assert!(execute(&store, &refs, &instance, end()).unwrap().reused);
}

#[test]
fn prelaunch_cancel_is_durable_and_repeat_does_not_issue_another_proof() {
    let (_temp, store, refs, instance, target) = fixture();
    assert!(!execute(&store, &refs, &instance, end()).unwrap().reused);
    let stopped = store.read().unwrap().instances.remove(0);
    assert_eq!(stopped.state, "stopped");
    assert!(load(&store, &refs, &target).unwrap().unwrap().outcome == Outcome::Completed);
    let before = std::fs::read_dir(store.root().join("references"))
        .unwrap()
        .count();
    assert!(execute(&store, &refs, &stopped, end()).unwrap().reused);
    assert_eq!(
        before,
        std::fs::read_dir(store.root().join("references"))
            .unwrap()
            .count()
    );
    assert!(!store.root().join("run-jobs").exists());
}

#[test]
fn interrupted_prelaunch_intent_and_completed_acknowledgement_recover() {
    for outcome in [Outcome::Pending, Outcome::Unknown, Outcome::Completed] {
        let (_temp, store, refs, mut instance, target) = fixture();
        save(
            &store,
            &refs,
            &Record {
                schema_version: 1,
                target,
                job: None,
                outcome,
            },
        )
        .unwrap();
        instance.state = "stopping".into();
        let mut tx = store.transaction(end()).unwrap();
        tx.registry.instances[0] = instance.clone();
        tx.commit().unwrap();
        let result = execute(&store, &refs, &instance, end()).unwrap();
        assert_eq!(result.reused, outcome == Outcome::Completed);
        assert_eq!(store.read().unwrap().instances[0].state, "stopped");
    }
}

#[test]
fn unsigned_stopping_or_unproven_running_instance_is_not_cancelled() {
    for phase in ["stopping", "starting", "ready", "stopped"] {
        let (_temp, store, refs, mut instance, _) = fixture();
        instance.state = phase.into();
        let mut tx = store.transaction(end()).unwrap();
        tx.registry.instances[0] = instance.clone();
        tx.commit().unwrap();
        assert_eq!(
            execute(&store, &refs, &instance, end()).err().unwrap().code,
            "STOP_UNVERIFIABLE"
        );
        assert_eq!(store.read().unwrap().instances[0], instance);
        assert!(!store.root().join("instance-stops").exists());
    }
}

#[test]
fn changed_installation_or_wrong_signer_cannot_reuse_completion() {
    let (_temp, store, refs, instance, _) = fixture();
    execute(&store, &refs, &instance, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    let other = References::with_key(store.root().join("references"), [18; 32]);
    assert_eq!(
        execute(&store, &other, &stopped, end()).err().unwrap().code,
        "REFERENCE_INVALID"
    );
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.installations[0].digest = "b".repeat(64);
    tx.commit().unwrap();
    assert!(execute(&store, &refs, &stopped, end()).is_err());
    assert_eq!(store.read().unwrap().instances[0], stopped);
}

#[test]
fn prior_dispatch_claim_prevents_prelaunch_cancellation_even_if_registry_was_rewound() {
    let (_temp, store, refs, instance, target) = fixture();
    let launch = super::super::start_intent::Target {
        instance_id: target.instance_id,
        run_id: target.run_id,
        installation_id: target.installation_id,
        installation_digest: target.installation_digest,
        input_hash: "b".repeat(64),
    };
    drop(super::super::start_intent::begin(&store, &refs, "test-request", &launch, end()).unwrap());
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = instance.clone();
    tx.commit().unwrap();
    assert_eq!(
        execute(&store, &refs, &instance, end()).err().unwrap().code,
        "STOP_UNVERIFIABLE"
    );
    assert!(!store.root().join("instance-stops").exists());
    assert_eq!(store.read().unwrap().instances[0], instance);
}

#[test]
fn selection_change_is_rejected_before_creating_stop_intent() {
    let (_temp, store, refs, instance, _) = fixture();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].run_id = "new-run".into();
    tx.commit().unwrap();
    assert_eq!(
        execute(&store, &refs, &instance, end()).err().unwrap().code,
        "STOP_INPUT_CHANGED"
    );
    assert!(!store.root().join("instance-stops").exists());
}

#[test]
fn cached_completion_commit_failure_is_unknown_and_reuses_the_same_proof_after_unlock() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temp, store, refs, original, target) = fixture();
    execute(&store, &refs, &original, end()).unwrap();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "stopping".into();
    tx.commit().unwrap();
    let stopping = store.read().unwrap().instances.remove(0);
    let path = store.root().join("registry.json");
    let before = std::fs::read(&path).unwrap();
    let proof = std::fs::read_dir(store.root().join("instance-stops"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let proof_bytes = std::fs::read(&proof).unwrap();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let error = execute(&store, &refs, &stopping, end()).err().unwrap();
    assert_eq!(error.code, "STOP_OUTCOME_UNKNOWN");
    assert_eq!(error.outcome, Some("unknown"));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(load(&store, &refs, &target).unwrap().unwrap().outcome == Outcome::Completed);
    assert_eq!(std::fs::read(&proof).unwrap(), proof_bytes);
    drop(held);
    assert!(execute(&store, &refs, &stopping, end()).unwrap().reused);
    assert_eq!(std::fs::read(proof).unwrap(), proof_bytes);
}

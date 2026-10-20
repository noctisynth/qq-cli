use super::*;
use crate::napcat::runtime::stop::{self, tests::fixture};
use std::time::Duration;
fn end() -> Instant {
    Instant::now() + Duration::from_secs(3)
}

#[test]
fn changed_snapshot_after_completion_never_overwrites_current_registration() {
    for field in 0..15 {
        let (_temp, store, refs, original, _target) = fixture();
        stop::execute(&store, &refs, &original, end()).unwrap();
        let registry = store.read().unwrap();
        let expected = Snapshot::new(&original, &registry.installations[0]);
        let proof = std::fs::read_dir(store.root().join("instance-stops"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let proof_bytes = std::fs::read(&proof).unwrap();
        let mut tx = store.transaction(end()).unwrap();
        tx.registry.instances[0].state = "stopping".into();
        match field {
            0 => tx.registry.instances[0].account = Some("10001".into()),
            1 => tx.registry.instances[0].endpoint = "http://127.0.0.1:19002/".into(),
            2 => tx.registry.instances[0].credential_key = "changed-key".into(),
            3 => tx.registry.instances[0].process_id = Some(123),
            4 => {
                tx.registry.instances[0].process_started =
                    Some("changed-process-observation".into())
            }
            5 => tx.registry.instances[0].run_id = "changed-run".into(),
            6 => tx.registry.instances[0].ws_endpoint = Some("ws://127.0.0.1:19003/event".into()),
            7 => tx.registry.installations[0].version = "changed-version".into(),
            8 => tx.registry.installations[0].directory = store.root().join("changed-installation"),
            9 => tx.registry.installations[0].digest = "b".repeat(64),
            10 => {
                let duplicate = tx.registry.installations[0].clone();
                tx.registry.installations.push(duplicate);
            }
            11 => tx.registry.instances[0].state = "ready".into(),
            12 => tx.registry.instances.clear(),
            13 => tx.registry.instances[0].installation_id = "changed-installation".into(),
            _ => {
                let mut duplicate = tx.registry.installations[0].clone();
                duplicate.digest = "b".repeat(64);
                tx.registry.installations.push(duplicate);
            }
        }
        tx.commit().unwrap();
        let current = std::fs::read(store.root().join("registry.json")).unwrap();
        let error = complete(&store, &expected, end())
            .err()
            .unwrap_or_else(|| panic!("changed field {field} was acknowledged"));
        assert_eq!(error.code, "STOP_OUTCOME_UNKNOWN");
        assert_eq!(
            error.outcome,
            Some(crate::core::contracts::write::Outcome::Unknown)
        );
        assert!(
            serde_json::to_value(error.details.as_ref().unwrap())
                .unwrap()
                .get("state")
                .is_none()
        );
        assert_eq!(
            std::fs::read(store.root().join("registry.json")).unwrap(),
            current
        );
        assert_eq!(std::fs::read(proof).unwrap(), proof_bytes);
    }
}

#[test]
fn unchanged_snapshot_acknowledges_completed_proof_and_keeps_repeat_reusable() {
    let (_temp, store, refs, original, _target) = fixture();
    stop::execute(&store, &refs, &original, end()).unwrap();
    let registry = store.read().unwrap();
    let expected = Snapshot::new(&original, &registry.installations[0]);
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "stopping".into();
    tx.commit().unwrap();
    complete(&store, &expected, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    assert_eq!(stopped.state, "stopped");
    assert!(
        stop::execute(&store, &refs, &stopped, end())
            .unwrap()
            .reused
    );
}

#[test]
fn acknowledgement_commit_failure_preserves_completion_for_recovery() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temp, store, refs, original, _target) = fixture();
    stop::execute(&store, &refs, &original, end()).unwrap();
    let registry = store.read().unwrap();
    let expected = Snapshot::new(&original, &registry.installations[0]);
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "stopping".into();
    tx.commit().unwrap();
    let path = store.root().join("registry.json");
    let before = std::fs::read(&path).unwrap();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let error = complete(&store, &expected, end()).err().unwrap();
    assert_eq!(error.code, "STOP_OUTCOME_UNKNOWN");
    assert_eq!(
        error.outcome,
        Some(crate::core::contracts::write::Outcome::Unknown)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    drop(held);
    complete(&store, &expected, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    assert!(
        stop::execute(&store, &refs, &stopped, end())
            .unwrap()
            .reused
    );
}

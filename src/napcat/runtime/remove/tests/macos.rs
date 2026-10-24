use super::*;
use crate::storage::state::{atomic_json, private_dir};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[test]
fn windows_stop_receipt_cannot_authorize_macos_removal() {
    let (_temp, store, refs, mut instance) = fixture();
    instance.state = "stopped".into();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = instance.clone();
    tx.commit().unwrap();
    let directory = store.root().join("instance-stops");
    private_dir(&directory).unwrap();
    let reference = refs.issue("p1", &json!({"schemaVersion":1,"target":{"instanceId":instance.id,"runId":instance.run_id,"installationId":instance.installation_id,"installationDigest":"a".repeat(64)},"job":null,"outcome":"completed"})).unwrap();
    let name = format!(
        "{:x}.json",
        Sha256::digest(serde_json::to_vec(&(&instance.id, &instance.run_id)).unwrap())
    );
    atomic_json(
        &directory.join(name),
        &json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "STOP_PROOF_REQUIRED"
    );
    assert_eq!(store.read().unwrap().instances, [instance]);
    assert!(!store.root().join("instance-removals").exists());
}

#[test]
fn completed_stop_binds_full_instance_and_installation_snapshot() {
    for field in ["endpoint", "credential", "installation"] {
        let (_temp, store, refs, instance) = fixture();
        platform_stop::execute(&store, &refs, &instance, end()).unwrap();
        let mut tx = store.transaction(end()).unwrap();
        match field {
            "endpoint" => tx.registry.instances[0].endpoint = "http://127.0.0.1:19009".into(),
            "credential" => tx.registry.instances[0].credential_key = "other-synthetic-key".into(),
            "installation" => {
                tx.registry.installations[0].directory = store.root().join("other-installation")
            }
            _ => unreachable!(),
        }
        let expected = tx.registry.instances[0].clone();
        tx.commit().unwrap();
        assert_eq!(
            execute(&store, &refs, &instance.id, end())
                .err()
                .unwrap()
                .code,
            "STOP_PROOF_REQUIRED"
        );
        assert_eq!(store.read().unwrap().instances, [expected]);
    }
}

#[test]
fn pending_stop_receipt_never_replaces_completed_proof() {
    let (_temp, store, refs, instance) = fixture();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    let path = std::fs::read_dir(store.root().join("macos-stops"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut pointer: Value = crate::storage::config::read_json(&path, 4096).unwrap();
    let mut record: Value = refs
        .get("p2", pointer["reference"].as_str().unwrap())
        .unwrap();
    record["completed"] = false.into();
    pointer["reference"] = refs.issue("p2", &record).unwrap().into();
    atomic_json(&path, &pointer).unwrap();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "STOP_PROOF_REQUIRED"
    );
    assert_eq!(store.read().unwrap().instances.len(), 1);
    assert!(!store.root().join("instance-removals").exists());
}

#[test]
fn macos_default_retirement_rejects_legacy_creation_authority_and_lost_guard() {
    use std::cell::Cell;
    for legacy in [true, false] {
        let (_temp, store, refs, instance) = fixture();
        let installation = store.read().unwrap().installations[0].clone();
        let directory = store.root().join("instance-creations");
        private_dir(&directory).unwrap();
        let reference=refs.issue(if legacy {"n1"} else {"n2"},&json!({"schemaVersion":1,"requestId":"default","installation":installation,"plan":{"instanceId":instance.id,"runId":instance.run_id,"version":"test","ports":{"http":19001,"events":19002,"control":19003}},"phase":"registered"})).unwrap();
        atomic_json(
            &directory.join(format!("{:x}.json", Sha256::digest(b"default"))),
            &json!({"schemaVersion":1,"reference":reference}),
        )
        .unwrap();
        let directory = store.root().join("automatic-start");
        private_dir(&directory).unwrap();
        let reference = refs
            .issue(
                "d1",
                &json!({"schemaVersion":1,"requestId":"default","version":"test"}),
            )
            .unwrap();
        let path = directory.join("active.json");
        atomic_json(&path, &json!({"schemaVersion":1,"reference":reference})).unwrap();
        let original = std::fs::read(&path).unwrap();
        let calls = Cell::new(0);
        let guard = || {
            calls.set(calls.get() + 1);
            if !legacy && calls.get() == 3 {
                return Err(AppError::storage());
            }
            Ok(())
        };
        assert!(
            crate::napcat::runtime::default_start::retire(&store, &refs, &instance, &guard)
                .is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

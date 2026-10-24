use super::*;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::napcat::runtime::macos::stop as platform_stop;
#[cfg(windows)]
use crate::napcat::runtime::stop as platform_stop;
use crate::storage::state::{Installation, Instance};
use std::time::Duration;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "tests/macos.rs"]
mod macos;
fn end() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
fn fixture() -> (tempfile::TempDir, Store, References, Instance) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [83; 32]);
    let instance = Instance {
        id: "test-instance".into(),
        run_id: "test-run".into(),
        installation_id: "test-installation".into(),
        endpoint: "http://127.0.0.1:19001".into(),
        ws_endpoint: None,
        credential_key: "test-credential".into(),
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.push(instance.clone());
    tx.registry.installations.push(Installation {
        id: instance.installation_id.clone(),
        version: "test".into(),
        digest: "a".repeat(64),
        directory: temp.path().canonicalize().unwrap().join("installation"),
    });
    tx.commit().unwrap();
    (temp, store, refs, instance)
}
#[test]
fn removal_requires_signed_stop_and_preserves_material_and_history() {
    let (_temp, store, refs, instance) = fixture();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "STOP_PROOF_REQUIRED"
    );
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "stopped".into();
    tx.commit().unwrap();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "STOP_PROOF_REQUIRED"
    );
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0] = instance.clone();
    tx.commit().unwrap();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    let marker = store.root().join("original-request.json");
    std::fs::write(&marker, b"preserve unknown receipt").unwrap();
    assert!(!execute(&store, &refs, &instance.id, end()).unwrap().reused);
    assert!(store.read().unwrap().instances.is_empty());
    assert_eq!(store.read().unwrap().installations.len(), 1);
    assert_eq!(std::fs::read(marker).unwrap(), b"preserve unknown receipt");
    let stops = if cfg!(windows) {
        "instance-stops"
    } else {
        "macos-stops"
    };
    assert!(store.root().join(stops).exists());
    assert!(execute(&store, &refs, &instance.id, end()).unwrap().reused);
    assert_eq!(
        execute(&store, &refs, "missing-instance", end())
            .err()
            .unwrap()
            .code,
        "INSTANCE_NOT_FOUND"
    );
}
#[test]
fn committed_removal_with_pending_receipt_recovers_after_installation_is_gone() {
    let (_temp, store, refs, instance) = fixture();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    let instance = store.read().unwrap().instances.remove(0);
    journal::save(
        &store,
        &refs,
        &Record {
            schema_version: 1,
            instance: instance.clone(),
            installation_digest: "a".repeat(64),
            outcome: Outcome::Pending,
        },
    )
    .unwrap();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.clear();
    tx.registry.installations.clear();
    tx.commit().unwrap();
    execute(&store, &refs, &instance.id, end()).unwrap();
    assert!(execute(&store, &refs, &instance.id, end()).unwrap().reused);
    assert!(store.read().unwrap().installations.is_empty());
}
#[test]
fn removal_never_adopts_a_recreated_registration_with_the_old_id() {
    let (_temp, store, refs, instance) = fixture();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    execute(&store, &refs, &instance.id, end()).unwrap();
    let mut replacement = instance.clone();
    replacement.run_id = "replacement-run".into();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances.push(replacement.clone());
    tx.commit().unwrap();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "REMOVE_INPUT_CHANGED"
    );
    assert_eq!(store.read().unwrap().instances[0], replacement);
}

#[test]
fn removal_retires_only_its_exact_default_identity_and_keeps_signed_history() {
    use serde_json::json;
    use sha2::{Digest, Sha256};
    for (id, run, matches) in [
        ("test-instance", "test-run", true),
        ("test-instance", "other-run", false),
        ("other-instance", "test-run", false),
    ] {
        let (_temp, store, refs, instance) = fixture();
        let creation = store.root().join("instance-creations");
        crate::storage::state::private_dir(&creation).unwrap();
        let installation = store.read().unwrap().installations.remove(0);
        let kind = if cfg!(windows) { "n1" } else { "n2" };
        let reference=refs.issue(kind, &json!({"schemaVersion":1,"requestId":"old-default","installation":installation,"plan":{"instanceId":id,"runId":run,"version":"test","ports":{"http":19001,"events":19002,"control":19003}},"phase":"registered"})).unwrap();
        let creation_path = creation.join(format!("{:x}.json", Sha256::digest(b"old-default")));
        crate::storage::state::atomic_json(
            &creation_path,
            &json!({"schemaVersion":1,"reference":reference}),
        )
        .unwrap();
        let directory = store.root().join("automatic-start");
        crate::storage::state::private_dir(&directory).unwrap();
        let reference = refs
            .issue(
                "d1",
                &json!({"schemaVersion":1,"requestId":"old-default","version":"test"}),
            )
            .unwrap();
        let path = directory.join("active.json");
        crate::storage::state::atomic_json(
            &path,
            &json!({"schemaVersion":1,"reference":reference}),
        )
        .unwrap();
        let original = std::fs::read(&path).unwrap();
        let creation_bytes = std::fs::read(&creation_path).unwrap();
        platform_stop::execute(&store, &refs, &instance, end()).unwrap();
        execute(&store, &refs, &instance.id, end()).unwrap();
        assert_eq!(path.exists(), !matches);
        assert_eq!(std::fs::read(&creation_path).unwrap(), creation_bytes);
        if matches {
            let super::super::default_start::Selection::Create { request_id } =
                super::super::default_start::select(&store, &refs, "test", end()).unwrap()
            else {
                panic!("new default expected")
            };
            assert_ne!(request_id, "old-default");
            let active = std::fs::read(&path).unwrap();
            assert!(execute(&store, &refs, &instance.id, end()).unwrap().reused);
            assert_eq!(std::fs::read(&path).unwrap(), active);
        } else {
            assert_eq!(std::fs::read(&path).unwrap(), original);
        }
    }
}

#[test]
fn missing_default_creation_proof_preserves_registration_and_active_pointer() {
    let (_temp, store, refs, instance) = fixture();
    platform_stop::execute(&store, &refs, &instance, end()).unwrap();
    let directory = store.root().join("automatic-start");
    crate::storage::state::private_dir(&directory).unwrap();
    let reference = refs.issue("d1", &serde_json::json!({"schemaVersion":1,"requestId":"missing-default-creation","version":"test"})).unwrap();
    let path = directory.join("active.json");
    crate::storage::state::atomic_json(
        &path,
        &serde_json::json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    let original = std::fs::read(&path).unwrap();
    assert_eq!(
        execute(&store, &refs, &instance.id, end())
            .err()
            .unwrap()
            .code,
        "AUTOMATIC_START_CONFLICT"
    );
    assert_eq!(store.read().unwrap().instances.len(), 1);
    assert_eq!(std::fs::read(path).unwrap(), original);
}

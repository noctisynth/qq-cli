use super::*;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn target(key: &Key) -> Target {
    Target {
        instance_id: "instance".into(),
        run_id: "run".into(),
        installation_id: "install".into(),
        installation_digest: key.archive().into(),
        input_hash: "e".repeat(64),
    }
}
#[test]
fn signed_exact_records_are_idempotent_and_never_downgrade_to_windows_or_a_new_key() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [83; 32]);
    let key = Key::current().unwrap();
    assert!(!supported(&store, &refs, &key).unwrap());
    assert!(!store.root().exists());
    let tx = store.transaction(deadline()).unwrap();
    assert!(record(&tx, &refs, key.clone(), target(&key), &|| Err(unverified())).is_err());
    assert!(!present(&store).unwrap());
    let mut bad = target(&key);
    bad.installation_digest = "f".repeat(64);
    assert!(record(&tx, &refs, key.clone(), bad, &|| Ok(())).is_err());
    record(&tx, &refs, key.clone(), target(&key), &|| Ok(())).unwrap();
    let files = std::fs::read_dir(store.root().join("references"))
        .unwrap()
        .count();
    record(&tx, &refs, key.clone(), target(&key), &|| Ok(())).unwrap();
    assert_eq!(
        std::fs::read_dir(store.root().join("references"))
            .unwrap()
            .count(),
        files
    );
    assert!(supported(&store, &refs, &key).unwrap());
    let mut value = serde_json::to_value(&key).unwrap();
    value["kernelRelease"] = "changed".into();
    let changed: Key = serde_json::from_value(value).unwrap();
    assert!(!supported(&store, &refs, &changed).unwrap());
    let pointer: Pointer = read_json(
        &store
            .root()
            .join(DIRECTORY)
            .join(format!("{}.json", key.fingerprint().unwrap())),
        4096,
    )
    .unwrap();
    assert!(refs.get::<Record>("k1", &pointer.reference).is_err());
    let wrong = References::with_key(store.root().join("references"), [84; 32]);
    assert!(supported(&store, &wrong, &key).is_err());
    drop(tx);
    // A surviving index alone must prevent replacing a lost signing authority.
    std::fs::remove_dir_all(store.root().join("references")).unwrap();
    struct Missing(Cell<usize>);
    impl crate::storage::credentials::Credentials for Missing {
        fn get(&self, _: &str) -> Result<String> {
            Err(AppError::new(
                "CREDENTIAL_NOT_FOUND",
                "Synthetic key absent",
                4,
            ))
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("no deletion")
        }
    }
    let missing = Missing(Cell::new(0));
    assert_eq!(
        References::open(&store, &missing, deadline())
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(missing.0.get(), 0);
}
#[test]
fn unknown_entries_and_corruption_are_preserved_and_block_new_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [85; 32]);
    let key = Key::current().unwrap();
    let tx = store.transaction(deadline()).unwrap();
    private_dir(&store.root().join(DIRECTORY)).unwrap();
    let foreign = store.root().join(DIRECTORY).join("foreign.json");
    std::fs::write(&foreign, b"preserve unknown material").unwrap();
    assert!(record(&tx, &refs, key.clone(), target(&key), &|| Ok(())).is_err());
    assert_eq!(
        std::fs::read(&foreign).unwrap(),
        b"preserve unknown material"
    );
    assert!(!store.root().join("references").exists());
}

use super::*;
#[test]
fn registry_maintenance_marker_requires_original_key_even_when_index_directories_are_gone() {
    struct Missing(std::cell::Cell<usize>);
    impl Credentials for Missing {
        fn get(&self, _: &str) -> Result<String> {
            Err(AppError::new(
                "CREDENTIAL_NOT_FOUND",
                "Original test key is absent.",
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
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let end = Instant::now() + std::time::Duration::from_secs(3);
    let mut tx = store.transaction(end).unwrap();
    tx.registry.instances.push(crate::storage::state::Instance {
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
    });
    tx.registry
        .maintenance_intents
        .insert("original-instance".into(), "v1.original-missing".into());
    tx.commit().unwrap();
    let missing = Missing(std::cell::Cell::new(0));
    assert_eq!(
        References::open(&store, &missing, end).err().unwrap().code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(missing.0.get(), 0);
    assert!(!store.root().join("references").exists());
}

use super::*;
struct NoSecrets;
impl Credentials for NoSecrets {
    fn get(&self, _: &str) -> Result<String> {
        panic!("no vault access")
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("no vault write")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no vault deletion")
    }
}
#[test]
fn explicit_trial_does_not_override_corrupt_existing_compatibility_material() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let directory = store.root().join("macos-compatibility");
    crate::storage::state::private_dir(&directory).unwrap();
    let foreign = directory.join("foreign.json");
    std::fs::write(&foreign, b"preserve unknown evidence").unwrap();
    let refs = References::with_key(store.root().join("references"), [87; 32]);
    let context = Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    for trial in [false, true] {
        let result = execute_with(
            &context,
            Options {
                instance: None,
                version: None,
                request: Some("original"),
                new: true,
                verify_compatibility: trial,
            },
            Instant::now() + std::time::Duration::from_secs(5),
            |_, _, _| panic!("corrupt evidence must stop before preparation"),
        );
        assert_eq!(result.err().unwrap().code, "STATE_INVALID");
    }
    assert_eq!(
        std::fs::read(&foreign).unwrap(),
        b"preserve unknown evidence"
    );
    assert!(!store.root().join("references").exists());
    assert!(!store.root().join("start-requests").exists());
}
#[test]
fn composed_preparation_can_acquire_its_lifecycle_lock_and_errors_keep_the_start_key() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [60; 32]);
    let context = Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    let deadline = Instant::now() + std::time::Duration::from_secs(1);
    let result = execute_with(
        &context,
        Options {
            instance: None,
            version: None,
            request: Some("original"),
            new: true,
            verify_compatibility: true,
        },
        deadline,
        |context, request, deadline| {
            assert_eq!(request, "original");
            let lock = crate::napcat::runtime::lifecycle_lock(context.store, deadline)?;
            lock.verify()?;
            Err(AppError::new(
                "SYNTHETIC_PREPARATION",
                "Synthetic preparation stopped before any effect",
                4,
            ))
        },
    );
    let error = result.err().unwrap();
    assert_eq!(error.code, "SYNTHETIC_PREPARATION");
    assert_eq!(error.request_id.as_deref(), Some("original"));
    assert!(!store.root().join("start-requests").exists());
    assert!(store.read().unwrap().instances.is_empty());
}

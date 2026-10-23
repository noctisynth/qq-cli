use super::*;
use std::cell::RefCell;
use std::time::Duration;
#[derive(Default)]
struct Memory {
    key: RefCell<Option<String>>,
    writes: std::cell::Cell<usize>,
}
impl Credentials for Memory {
    fn get(&self, _: &str) -> Result<String> {
        self.key
            .borrow()
            .clone()
            .ok_or_else(|| AppError::new("CREDENTIAL_NOT_FOUND", "Original test key absent.", 4))
    }
    fn set(&self, _: &str, key: &str) -> Result<()> {
        *self.key.borrow_mut() = Some(key.into());
        self.writes.set(self.writes.get() + 1);
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no deletion")
    }
}
#[test]
fn lost_signer_never_replaces_existing_references_or_lifecycle_journals() {
    let deadline = || Instant::now() + Duration::from_secs(5);
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let memory = Memory::default();
    let refs = References::open(&store, &memory, deadline()).unwrap();
    let reference = refs
        .issue("m1", &serde_json::json!({"original":"locator"}))
        .unwrap();
    memory.key.borrow_mut().take();
    assert_eq!(
        References::open(&store, &memory, deadline())
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(memory.writes.get(), 1);
    assert_eq!(
        refs.get::<serde_json::Value>("m1", &reference).unwrap()["original"],
        "locator"
    );
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let directory = store.root().join("run-jobs");
    private_dir(&directory).unwrap();
    let marker = directory.join("original-journal.json");
    std::fs::write(&marker, b"original journal").unwrap();
    let memory = Memory::default();
    assert_eq!(
        References::open(&store, &memory, deadline())
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(memory.writes.get(), 0);
    assert_eq!(std::fs::read(marker).unwrap(), b"original journal");
}

#[test]
fn lost_signer_does_not_replace_creation_journal_authority() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let directory = store.root().join("instance-creations");
    private_dir(&directory).unwrap();
    let marker = directory.join("original-claim.json");
    std::fs::write(&marker, b"original claim").unwrap();
    let memory = Memory::default();
    assert_eq!(
        References::open(&store, &memory, Instant::now() + Duration::from_secs(1))
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(memory.writes.get(), 0);
    assert_eq!(std::fs::read(marker).unwrap(), b"original claim");
}

#[test]
fn automatic_start_checkpoint_requires_its_original_signer() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let directory = store.root().join("automatic-start");
    private_dir(&directory).unwrap();
    let marker = directory.join("active.json");
    std::fs::write(&marker, b"original automatic checkpoint").unwrap();
    let memory = Memory::default();
    assert_eq!(
        References::open(&store, &memory, Instant::now() + Duration::from_secs(2))
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(memory.writes.get(), 0);
    assert_eq!(
        std::fs::read(marker).unwrap(),
        b"original automatic checkpoint"
    );
}

#[test]
fn macos_run_pointer_without_reference_files_still_requires_original_signer() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let directory = store.root().join("macos-runs");
    private_dir(&directory).unwrap();
    std::fs::write(directory.join("original.json"), b"original pointer").unwrap();
    let memory = Memory::default();
    assert_eq!(
        References::open(&store, &memory, Instant::now() + Duration::from_secs(2))
            .err()
            .unwrap()
            .code,
        "SIGNING_KEY_REQUIRED"
    );
    assert_eq!(memory.writes.get(), 0);
}

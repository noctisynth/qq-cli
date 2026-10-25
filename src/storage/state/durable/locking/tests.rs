use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}

#[test]
fn lock_contention_is_bounded_and_original_release_allows_reacquisition() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let held = Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").unwrap();
    assert_eq!(
        Lock::acquire(
            &root,
            "state.lock",
            Instant::now() + Duration::from_millis(20),
            "STATE_BUSY"
        )
        .err()
        .unwrap()
        .code,
        "STATE_BUSY"
    );
    drop(held);
    Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").unwrap();
    assert_eq!(
        std::fs::metadata(root.join("state.lock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn lock_replacement_invalidates_original_guard_and_transaction() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let store = crate::storage::state::Store::new(root.clone());
    let transaction = store.transaction(deadline()).unwrap();
    std::fs::rename(root.join("state.lock"), root.join("old.lock")).unwrap();
    std::fs::write(root.join("state.lock"), b"replacement").unwrap();
    assert!(transaction.persist().is_err());
    assert!(!root.join("registry.json").exists());
    assert_eq!(
        std::fs::read(root.join("state.lock")).unwrap(),
        b"replacement"
    );
}

#[test]
fn root_replacement_never_receives_original_transaction_state() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("state");
    let store = crate::storage::state::Store::new(root.clone());
    let transaction = store.transaction(deadline()).unwrap();
    std::fs::rename(&root, root.with_extension("old")).unwrap();
    std::fs::create_dir(&root).unwrap();
    assert!(transaction.persist().is_err());
    assert!(!root.join("registry.json").exists());
    assert!(!root.with_extension("old").join("registry.json").exists());
}

#[test]
fn linked_or_shared_writable_locks_are_rejected_without_chmod() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::write(root.join("target"), b"unchanged").unwrap();
    symlink(root.join("target"), root.join("state.lock")).unwrap();
    assert!(Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").is_err());
    std::fs::remove_file(root.join("state.lock")).unwrap();
    std::fs::hard_link(root.join("target"), root.join("state.lock")).unwrap();
    assert!(Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").is_err());
    std::fs::remove_file(root.join("state.lock")).unwrap();
    std::fs::write(root.join("state.lock"), b"unchanged").unwrap();
    std::fs::set_permissions(
        root.join("state.lock"),
        std::fs::Permissions::from_mode(0o666),
    )
    .unwrap();
    assert!(Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").is_err());
    assert_eq!(
        std::fs::metadata(root.join("state.lock"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o666
    );
    assert_eq!(std::fs::read(root.join("target")).unwrap(), b"unchanged");
}

#[test]
fn shared_writable_root_is_rejected_before_lock_creation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o770)).unwrap();
    assert!(Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").is_err());
    assert!(!root.join("state.lock").exists());
}

#[test]
fn extended_acl_on_root_or_lock_invalidates_the_original_guard_without_repair() {
    use std::os::fd::AsFd;
    for root_acl in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let held = Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").unwrap();
        let path = if root_acl {
            root.clone()
        } else {
            root.join("state.lock")
        };
        assert!(
            std::process::Command::new("/bin/chmod")
                .args(["+a", "everyone allow write"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let file = File::open(&path).unwrap();
        let acl = crate::native::macos_acl::digest(file.as_fd()).unwrap();
        assert_ne!(acl, crate::native::macos_acl::absent_digest());
        assert!(held.verify().is_err());
        assert!(held.publish_registry(&true).is_err());
        assert!(!root.join("registry.json").exists());
        assert_eq!(crate::native::macos_acl::digest(file.as_fd()).unwrap(), acl);
        drop(held);
        assert!(Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").is_err());
    }
}

#[test]
fn lost_lock_during_serialization_refuses_registry_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let held = Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").unwrap();
    std::fs::write(root.join("registry.json"), b"original").unwrap();
    struct ReplaceLock(PathBuf);
    impl serde::Serialize for ReplaceLock {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            std::fs::rename(self.0.join("state.lock"), self.0.join("old.lock")).unwrap();
            std::fs::write(self.0.join("state.lock"), b"replacement").unwrap();
            serializer.serialize_bool(true)
        }
    }
    assert!(held.publish_registry(&ReplaceLock(root.clone())).is_err());
    assert_eq!(
        std::fs::read(root.join("registry.json")).unwrap(),
        b"original"
    );
}

#[test]
fn synthetic_lock_contender() {
    let Some(root) = std::env::var_os("QQ_CLI_SYNTHETIC_LOCK_ROOT") else {
        return;
    };
    let result = Lock::acquire(
        Path::new(&root),
        "state.lock",
        Instant::now() + Duration::from_millis(30),
        "STATE_BUSY",
    );
    if std::env::var_os("QQ_CLI_SYNTHETIC_LOCK_BUSY").is_some() {
        assert_eq!(result.err().unwrap().code, "STATE_BUSY");
    } else {
        result.unwrap().verify().unwrap();
    }
}

#[test]
fn separate_process_observes_the_same_lock_and_release() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let held = Lock::acquire(&root, "state.lock", deadline(), "STATE_BUSY").unwrap();
    let run = |busy: bool| {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "storage::state::durable::locking::tests::synthetic_lock_contender",
                "--test-threads=1",
            ])
            .env("QQ_CLI_SYNTHETIC_LOCK_ROOT", &root)
            .env_remove("QQ_CLI_SYNTHETIC_LOCK_BUSY");
        if busy {
            command.env("QQ_CLI_SYNTHETIC_LOCK_BUSY", "1");
        }
        assert!(command.output().unwrap().status.success());
    };
    run(true);
    drop(held);
    run(false);
}

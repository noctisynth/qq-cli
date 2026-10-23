use super::*;
use std::os::unix::fs::{PermissionsExt, symlink};

#[test]
fn new_directories_are_private_without_changing_existing_parents() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o750)).unwrap();
    private_dir(&root.join("first/second")).unwrap();
    assert_eq!(
        std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o750
    );
    for relative in ["first", "first/second"] {
        assert_eq!(
            std::fs::metadata(root.join(relative))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    std::fs::write(root.join("file"), b"unchanged").unwrap();
    assert!(private_dir(&root.join("file")).is_err());
    symlink(root.join("first"), root.join("alias")).unwrap();
    assert!(private_dir(&root.join("alias/third")).is_err());
    assert!(!root.join("first/third").exists());
}

#[test]
fn atomic_replacement_is_private_and_serialization_failure_preserves_original() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("state.json");
    atomic_json(&path, &serde_json::json!({"generation": 1})).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    struct Fails;
    impl serde::Serialize for Fails {
        fn serialize<S: serde::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("synthetic failure"))
        }
    }
    assert!(atomic_json(&path, &Fails).is_err());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "{\"generation\":1}\n"
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    atomic_json(&path, &serde_json::json!({"generation": 2})).unwrap();
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "{\"generation\":2}\n"
    );
}

#[test]
fn anchored_directory_refuses_replacement_and_link_targets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("original");
    std::fs::create_dir(&path).unwrap();
    let directory = Directory::open(&path).unwrap();
    let file = directory.create_new("original.json").unwrap();
    std::fs::rename(&path, root.join("moved")).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(directory.sync_publication(&file).is_err());
    assert!(directory.create_new("replacement.json").is_err());
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
    symlink(root.join("moved"), root.join("alias")).unwrap();
    assert!(Directory::open(&root.join("alias")).is_err());
    let target = path.join("target.json");
    std::fs::write(&target, b"untouched").unwrap();
    symlink(&target, path.join("link.json")).unwrap();
    assert!(atomic_json(&path.join("link.json"), &true).is_err());
    std::fs::hard_link(&target, path.join("hard.json")).unwrap();
    assert!(atomic_json(&path.join("hard.json"), &true).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"untouched");
}

#[test]
fn signed_reference_publication_uses_private_files() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap().join("references");
    let refs = crate::storage::references::References::with_key(directory.clone(), [75; 32]);
    let reference = refs.issue("j1", &42).unwrap();
    assert_eq!(refs.get::<u8>("j1", &reference).unwrap(), 42);
    let path = std::fs::read_dir(directory)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn directory_replacement_during_serialization_preserves_both_targets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("original");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("state.json"), b"old original").unwrap();
    struct ReplaceDirectory {
        path: PathBuf,
        moved: PathBuf,
    }
    impl serde::Serialize for ReplaceDirectory {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            std::fs::rename(&self.path, &self.moved).unwrap();
            std::fs::create_dir(&self.path).unwrap();
            std::fs::write(self.path.join("state.json"), b"replacement target").unwrap();
            serializer.serialize_bool(true)
        }
    }
    let value = ReplaceDirectory {
        path: path.clone(),
        moved: root.join("moved"),
    };
    assert!(atomic_json(&path.join("state.json"), &value).is_err());
    assert_eq!(
        std::fs::read(path.join("state.json")).unwrap(),
        b"replacement target"
    );
    assert_eq!(
        std::fs::read(value.moved.join("state.json")).unwrap(),
        b"old original"
    );
    assert_eq!(std::fs::read_dir(value.moved).unwrap().count(), 1);
}

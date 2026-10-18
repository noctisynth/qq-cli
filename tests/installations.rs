use std::io::{Cursor, Read, Write};
use std::time::{Duration, Instant};

use qq_cli::archive::Expected;
use qq_cli::catalog::Release;
use qq_cli::installation;
use qq_cli::references::References;
use qq_cli::state::{Instance, Store};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

mod common;
use common::synthetic_release as fixture;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
struct MustNotRead;
impl Read for MustNotRead {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("cached or unauthorized operation must not read archive input")
    }
}

#[test]
fn owned_installation_registers_idempotently_and_uninstall_preserves_unrelated_state() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [81; 32]);
    let (bytes, release) = fixture();
    let result = serde_json::to_value(
        installation::import(Cursor::new(&bytes), &release, &store, &refs, deadline()).unwrap(),
    )
    .unwrap();
    let id = result["data"]["installationId"].as_str().unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    assert_eq!(installed.id, id);
    installation::verify(&store, &installed, &refs, deadline()).unwrap();
    let cached: Value = serde_json::to_value(
        installation::import(MustNotRead, &release, &store, &refs, deadline()).unwrap(),
    )
    .unwrap();
    assert_eq!(cached["data"]["cached"], true);
    assert_eq!(store.read().unwrap().installations.len(), 1);
    let user_data = store.root().join("qq-data");
    std::fs::create_dir(&user_data).unwrap();
    std::fs::write(user_data.join("keep.txt"), b"synthetic user data").unwrap();
    let ledger = store.root().join("requests");
    std::fs::create_dir(&ledger).unwrap();
    std::fs::write(ledger.join("keep.json"), b"synthetic request record").unwrap();
    installation::uninstall(&store, id, &refs, deadline()).unwrap();
    assert!(store.read().unwrap().installations.is_empty());
    assert!(!installed.directory.exists());
    assert_eq!(
        std::fs::read(user_data.join("keep.txt")).unwrap(),
        b"synthetic user data"
    );
    assert_eq!(
        std::fs::read(ledger.join("keep.json")).unwrap(),
        b"synthetic request record"
    );
}

#[test]
fn stopped_instance_references_and_modified_unknown_files_prevent_uninstall() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [82; 32]);
    let (bytes, release) = fixture();
    installation::import(Cursor::new(bytes), &release, &store, &refs, deadline()).unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.instances.push(Instance {
        id: "synthetic-instance".into(),
        installation_id: installed.id.clone(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: None,
        run_id: "synthetic-run".into(),
        state: "stopped".into(),
        process_id: None,
        process_started: None,
    });
    transaction.commit().unwrap();
    assert_eq!(
        installation::uninstall(&store, &installed.id, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
    assert!(installed.directory.exists());
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.instances.clear();
    transaction.commit().unwrap();
    std::fs::write(
        installed.directory.join("user-note.txt"),
        b"synthetic unrelated note",
    )
    .unwrap();
    assert_eq!(
        installation::uninstall(&store, &installed.id, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_MODIFIED"
    );
    assert!(installed.directory.join("user-note.txt").exists());
    std::fs::remove_file(installed.directory.join("user-note.txt")).unwrap();
    std::fs::write(
        installed.directory.join("loader.exe"),
        b"modified synthetic loader",
    )
    .unwrap();
    assert_eq!(
        installation::verify(&store, &installed, &refs, deadline())
            .unwrap_err()
            .code,
        "INSTALLATION_MODIFIED"
    );
    assert_eq!(
        installation::uninstall(&store, &installed.id, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_MODIFIED"
    );
    assert_eq!(
        std::fs::read(installed.directory.join("loader.exe")).unwrap(),
        b"modified synthetic loader"
    );
}

#[test]
fn namespace_or_signature_mismatch_never_deletes_user_paths() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [83; 32]);
    let (bytes, release) = fixture();
    installation::import(Cursor::new(bytes), &release, &store, &refs, deadline()).unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let foreign = References::with_key(store.root().join("references"), [84; 32]);
    assert_eq!(
        installation::uninstall(&store, &installed.id, &foreign, deadline())
            .err()
            .unwrap()
            .code,
        "REFERENCE_INVALID"
    );
    let protected = temp.path().join("user-data");
    std::fs::create_dir(&protected).unwrap();
    std::fs::write(protected.join("keep.txt"), b"preserved").unwrap();
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.installations[0].directory = protected.clone();
    transaction.commit().unwrap();
    assert_eq!(
        installation::uninstall(&store, &installed.id, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_MODIFIED"
    );
    assert!(installed.directory.exists());
    assert_eq!(
        std::fs::read(protected.join("keep.txt")).unwrap(),
        b"preserved"
    );
}

#[test]
fn publication_before_registry_failure_recovers_only_the_original_owned_artifact() {
    struct Obstruct {
        input: Cursor<Vec<u8>>,
        path: Option<std::path::PathBuf>,
    }
    impl Read for Obstruct {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.input.read(buffer)?;
            if count == 0
                && let Some(path) = self.path.take()
            {
                std::fs::create_dir(path)?;
            }
            Ok(count)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [85; 32]);
    let (bytes, release) = fixture();
    let obstruction = store.root().join("registry.json");
    assert_eq!(
        installation::import(
            Obstruct {
                input: Cursor::new(bytes),
                path: Some(obstruction.clone())
            },
            &release,
            &store,
            &refs,
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "STATE_IO"
    );
    assert_eq!(
        std::fs::read_dir(store.root().join("installations"))
            .unwrap()
            .count(),
        1
    );
    std::fs::remove_dir(obstruction).unwrap();
    let result: Value = serde_json::to_value(
        installation::import(MustNotRead, &release, &store, &refs, deadline()).unwrap(),
    )
    .unwrap();
    assert_eq!(result["data"]["cached"], true);
    assert_eq!(store.read().unwrap().installations.len(), 1);
}

#[test]
fn unconfirmed_release_is_not_read_or_registered() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [86; 32]);
    let release = qq_cli::catalog::lookup("v4.18.38").unwrap();
    assert_eq!(
        installation::import(MustNotRead, &release, &store, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "LICENSE_UNCONFIRMED"
    );
    assert!(!store.root().exists());
}

#[test]
fn a_large_member_manifest_remains_authenticated_above_the_message_reference_budget() {
    let deadline = || Instant::now() + Duration::from_secs(30);
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [89; 32]);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..680 {
        writer
            .start_file(
                format!("native/synthetic-module-{index:04}.node"),
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"synthetic member").unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let digest = Box::leak(format!("{:x}", Sha256::digest(&bytes)).into_boxed_str());
    let release = Release {
        version: "v0.0.0",
        commit: "synthetic-owned-source",
        url: "https://invalid.example/synthetic-owned.zip",
        archive: Expected {
            bytes: bytes.len() as u64,
            sha256: digest,
            files: 680,
            required: &[],
        },
        authorization_evidence: Some("original synthetic fixture"),
    };
    installation::import(Cursor::new(bytes), &release, &store, &refs, deadline()).unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let marker: Value = serde_json::from_slice(
        &std::fs::read(installed.directory.join(".qq-cli-installation.json")).unwrap(),
    )
    .unwrap();
    let metadata: Value = refs
        .get("i1", marker["reference"].as_str().unwrap())
        .unwrap();
    assert!(serde_json::to_vec(&metadata).unwrap().len() > 64 * 1024);
    assert!(refs.issue("m1", &metadata).is_err());
    installation::verify(&store, &installed, &refs, deadline()).unwrap();
    installation::uninstall(&store, &installed.id, &refs, deadline()).unwrap();
    assert!(!installed.directory.exists());
}

#[cfg(windows)]
#[test]
fn interrupted_removal_resumes_a_remaining_subset_and_keeps_its_journal() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [87; 32]);
    let (bytes, release) = fixture();
    installation::import(Cursor::new(bytes), &release, &store, &refs, deadline()).unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(installed.directory.join("native/module.node"))
        .unwrap();
    assert!(installation::uninstall(&store, &installed.id, &refs, deadline()).is_err());
    let journal = store
        .root()
        .join("installation-operations")
        .join(format!("{}.json", installed.id));
    assert!(journal.exists());
    assert_eq!(store.read().unwrap().installations.len(), 1);
    drop(held);
    let trash = installed
        .directory
        .parent()
        .unwrap()
        .join(format!(".removing-{}", installed.id));
    if installed.directory.exists() {
        std::fs::rename(&installed.directory, &trash).unwrap();
    }
    // Model a process stopping after one authenticated member was removed.
    let first = trash.join("loader.exe");
    if first.exists() {
        std::fs::remove_file(first).unwrap();
    }
    installation::uninstall(&store, &installed.id, &refs, deadline()).unwrap();
    assert!(!trash.exists());
    assert!(!journal.exists());
    assert!(store.read().unwrap().installations.is_empty());
}

#[cfg(windows)]
#[test]
fn registry_commit_failure_after_physical_removal_can_be_retried_without_data_loss() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [88; 32]);
    let (bytes, release) = fixture();
    installation::import(Cursor::new(bytes), &release, &store, &refs, deadline()).unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let held = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(store.root().join("registry.json"))
        .unwrap();
    assert_eq!(
        installation::uninstall(&store, &installed.id, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "STATE_IO"
    );
    assert!(!installed.directory.exists());
    assert_eq!(store.read().unwrap().installations.len(), 1);
    drop(held);
    installation::uninstall(&store, &installed.id, &refs, deadline()).unwrap();
    assert!(store.read().unwrap().installations.is_empty());
}

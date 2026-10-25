//! Pending macOS status must reach authenticated process/control observation.
use super::*;
use clap::Parser;
use sha2::{Digest, Sha256};
use std::{fs, time::Duration};

fn registry(state: &str) -> Registry {
    Registry {
        instances: vec![Instance {
            id: "synthetic-instance".into(),
            installation_id: "synthetic-installation".into(),
            endpoint: "http://127.0.0.1:16000/".into(),
            ws_endpoint: None,
            credential_key: "synthetic-api".into(),
            account: None,
            run_id: "synthetic-run".into(),
            state: state.into(),
            process_id: None,
            process_started: None,
        }],
        ..Registry::default()
    }
}

fn status(store: &Store, registry: &Registry) -> Result<Resolution> {
    resolve(
        &Cli::parse_from(["qq-cli", "status"]),
        store,
        registry,
        Instant::now() + Duration::from_secs(3),
    )
}

fn pointer(store: &Store, registry: &Registry) -> std::path::PathBuf {
    let instance = &registry.instances[0];
    let digest = Sha256::digest(serde_json::to_vec(&(&instance.id, &instance.run_id)).unwrap());
    store
        .root()
        .join("macos-runs")
        .join(format!("{digest:x}.json"))
}

#[test]
fn pending_macos_run_routes_to_live_observation_instead_of_cached_login_success() {
    for state in ["starting", "login_required"] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
        let registry = registry(state);
        let path = pointer(&store, &registry);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Presence only selects the authenticated path. Even an invalid record
        // must not produce a successful cached login status before validation.
        fs::write(path, b"invalid synthetic run pointer").unwrap();
        let Resolution::Selected(selected) = status(&store, &registry).unwrap() else {
            panic!("macOS pending run bypassed live observation");
        };
        assert_eq!(selected.instance, registry.instances[0]);
        assert!(selected.references.is_none());
    }
}

#[test]
fn status_without_a_run_remains_offline_and_does_not_create_state() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("absent-state"));
    for state in ["starting", "login_required", "stopped"] {
        assert!(matches!(
            status(&store, &registry(state)).unwrap(),
            Resolution::Immediate(_)
        ));
        assert!(!store.root().exists());
    }
}

#[test]
fn pending_status_rejects_a_linked_macos_run_pointer() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let registry = registry("login_required");
    let path = pointer(&store, &registry);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let target = temp.path().join("foreign-record");
    fs::write(&target, b"foreign synthetic run pointer").unwrap();
    symlink(&target, path).unwrap();
    assert!(status(&store, &registry).is_err());
    assert_eq!(fs::read(target).unwrap(), b"foreign synthetic run pointer");
}

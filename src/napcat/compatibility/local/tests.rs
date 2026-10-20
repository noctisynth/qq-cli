use super::*;
use crate::native::platform::Host;
use std::time::{Duration, Instant};
fn key() -> Key {
    Key {
        host: Host {
            os: "windows".into(),
            os_build: Some(10000),
            native_arch: "x86_64".into(),
            process_arch: "x86_64".into(),
        },
        mode: "framework_native".into(),
        napcat_version: "v4.18.38".into(),
        commit: "a".repeat(40),
        archive_sha256: "a".repeat(64),
        launcher_sha256: "b".repeat(64),
        loader_sha256: "c".repeat(64),
        qq_version: "9.9.99".into(),
        qq_installation_id: "synthetic-qq".into(),
        qq_build: "99999".into(),
        qq_architecture: "x86_64".into(),
        qq_fingerprint: "d".repeat(64),
    }
}
fn source() -> Target {
    Target {
        instance_id: "instance-example".into(),
        run_id: "run-example".into(),
        installation_id: "install-example".into(),
        installation_digest: "a".repeat(64),
        input_hash: "e".repeat(64),
    }
}
#[test]
fn local_observations_are_signed_exact_and_idempotent_without_body_or_secrets() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [61; 32]);
    assert!(load(&store, &refs).unwrap().is_empty());
    assert!(!store.root().exists());
    {
        let tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        record(&tx, &refs, key(), source()).unwrap();
        record(&tx, &refs, key(), source()).unwrap();
    }
    let witnesses = load(&store, &refs).unwrap();
    assert_eq!(witnesses.len(), 1);
    assert_eq!(
        super::super::evaluate(&key(), &witnesses).state,
        Verdict::Supported
    );
    let mut changed = key();
    changed.qq_fingerprint = "f".repeat(64);
    assert_eq!(
        super::super::evaluate(&changed, &witnesses).state,
        Verdict::Unknown
    );
    let wrong = References::with_key(store.root().join("references"), [62; 32]);
    assert!(load(&store, &wrong).is_err());
    let path = store
        .root()
        .join("compatibility-evidence")
        .join(format!("{}.json", key().fingerprint()));
    let mut pointer: serde_json::Value = read_json(&path, 4096).unwrap();
    pointer["schemaVersion"] = serde_json::json!(2);
    atomic_json(&path, &pointer).unwrap();
    assert!(load(&store, &refs).is_err());
}
#[test]
fn local_observations_preserve_unknown_entries_and_reject_digest_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [63; 32]);
    let tx = store
        .transaction(Instant::now() + Duration::from_secs(5))
        .unwrap();
    let mut bad = source();
    bad.installation_digest = "f".repeat(64);
    assert!(record(&tx, &refs, key(), bad).is_err());
    let directory = store.root().join("compatibility-evidence");
    private_dir(&directory).unwrap();
    let unknown = directory.join("foreign.json");
    std::fs::write(&unknown, b"foreign").unwrap();
    assert!(record(&tx, &refs, key(), source()).is_err());
    assert_eq!(std::fs::read(unknown).unwrap(), b"foreign");
}

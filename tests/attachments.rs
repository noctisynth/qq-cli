use qq_cli::messaging::attachments::{self, Kind};
use qq_cli::storage::state::Store;
use std::time::{Duration, Instant};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn snapshots_preserve_bytes_and_stable_fingerprints_across_staging_paths() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("original.bin");
    std::fs::write(&source, b"synthetic attachment").unwrap();
    let store = Store::new(temp.path().join("state"));
    let first = attachments::prepare(&source, Kind::File, &store, deadline()).unwrap();
    let second = attachments::prepare(&source, Kind::File, &store, deadline()).unwrap();
    assert_ne!(first.path(), second.path());
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert!(
        !serde_json::to_string(&first.fingerprint())
            .unwrap()
            .contains("state")
    );
    std::fs::write(&source, b"changed input").unwrap();
    assert_eq!(
        std::fs::read(first.path()).unwrap(),
        b"synthetic attachment"
    );
    let prepared = second.path().to_owned();
    drop(second);
    assert!(!prepared.exists());
    let refs =
        qq_cli::storage::references::References::with_key(temp.path().join("refs"), [85; 32]);
    let scope = qq_cli::core::domain::Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    };
    let retained = first
        .retain(&store, &refs, "attachment-intent", &scope, deadline())
        .unwrap();
    assert_eq!(
        std::fs::read(retained.path()).unwrap(),
        b"synthetic attachment"
    );
    let path = retained.path().to_owned();
    drop(retained);
    assert!(path.exists());
    let changed = attachments::prepare(&source, Kind::File, &store, deadline()).unwrap();
    assert_ne!(
        changed.fingerprint().sha256,
        format!("{:x}", sha2::Sha256::digest(b"synthetic attachment"))
    );
}

use sha2::Digest;
#[test]
fn invalid_attachment_inputs_fail_before_creating_staging_state() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let empty = temp.path().join("empty.bin");
    std::fs::write(&empty, []).unwrap();
    for path in [
        empty,
        temp.path().join("missing.bin"),
        temp.path().to_owned(),
    ] {
        assert_eq!(
            attachments::prepare(&path, Kind::Image, &store, deadline())
                .err()
                .unwrap()
                .code,
            "ATTACHMENT_INPUT_INVALID"
        );
    }
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"fixture").unwrap();
    assert_eq!(
        attachments::prepare(&source, Kind::File, &store, Instant::now())
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    assert!(!store.root().exists());
}

#[test]
fn retained_attachments_recover_from_disk_only_with_matching_request_scope_and_bytes() {
    use qq_cli::core::domain::Scope;
    use qq_cli::storage::references::References;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"synthetic retained bytes").unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [86; 32]);
    let scope = Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    };
    let retained = attachments::prepare(&source, Kind::File, &store, deadline())
        .unwrap()
        .retain(&store, &refs, "retained-request", &scope, deadline())
        .unwrap();
    let directory = retained.directory().to_owned();
    let fingerprint = retained.fingerprint().clone();
    drop(retained);
    drop(refs);
    let refs = References::with_key(temp.path().join("refs"), [86; 32]);
    let restored = attachments::load(
        &directory,
        &store,
        &refs,
        "retained-request",
        &scope,
        deadline(),
    )
    .unwrap();
    assert_eq!(restored.fingerprint(), &fingerprint);
    assert_eq!(
        std::fs::read(restored.path()).unwrap(),
        b"synthetic retained bytes"
    );
    assert!(
        attachments::load(
            &directory,
            &store,
            &refs,
            "another-request",
            &scope,
            deadline()
        )
        .is_err()
    );
    let other = Scope {
        account: "10002".into(),
        ..scope.clone()
    };
    assert!(
        attachments::load(
            &directory,
            &store,
            &refs,
            "retained-request",
            &other,
            deadline()
        )
        .is_err()
    );
    std::fs::write(restored.path(), b"synthetic modified bytes").unwrap();
    assert!(
        attachments::load(
            &directory,
            &store,
            &refs,
            "retained-request",
            &scope,
            deadline()
        )
        .is_err()
    );
}

#[test]
fn unsigned_foreign_or_extra_attachment_files_are_never_adopted() {
    use qq_cli::core::domain::Scope;
    use qq_cli::storage::references::References;
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"synthetic bytes").unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [87; 32]);
    let scope = Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    };
    let retained = attachments::prepare(&source, Kind::Image, &store, deadline())
        .unwrap()
        .retain(&store, &refs, "request", &scope, deadline())
        .unwrap();
    std::fs::write(retained.directory().join("unowned.bin"), b"foreign").unwrap();
    assert!(
        attachments::load(
            retained.directory(),
            &store,
            &refs,
            "request",
            &scope,
            deadline()
        )
        .is_err()
    );
    assert!(attachments::load(temp.path(), &store, &refs, "request", &scope, deadline()).is_err());
    std::fs::remove_file(retained.directory().join("unowned.bin")).unwrap();
    let marker = retained.directory().join(".qq-cli-attachment.json");
    let mut pointer: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
    pointer["reference"] = serde_json::json!("a1.invalid.invalid");
    std::fs::write(&marker, serde_json::to_vec(&pointer).unwrap()).unwrap();
    assert!(
        attachments::load(
            retained.directory(),
            &store,
            &refs,
            "request",
            &scope,
            deadline()
        )
        .is_err()
    );
    assert!(retained.path().exists());
}

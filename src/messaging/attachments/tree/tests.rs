use super::*;
use std::time::{Duration, SystemTime};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn instance() -> crate::storage::state::Instance {
    crate::storage::state::Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:9/".into(),
        ws_endpoint: None,
        credential_key: "synthetic-tree".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}
fn scope() -> Scope {
    Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    }
}
#[test]
fn directory_snapshots_are_stable_content_bound_retained_and_not_granted_by_quota_tags() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("input");
    std::fs::create_dir_all(root.join("nested/empty")).unwrap();
    std::fs::write(root.join("first.txt"), b"first").unwrap();
    std::fs::write(root.join("nested/second.txt"), b"second").unwrap();
    let store = Store::new(t.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [7; 32]);
    let first = prepare(&root, &store, &instance(), deadline()).unwrap();
    let second = prepare(&root, &store, &instance(), deadline()).unwrap();
    assert_eq!(
        first.logical_fingerprint().unwrap(),
        second.logical_fingerprint().unwrap()
    );
    assert_eq!(first.logical_fingerprint().unwrap()["kind"], "directory");
    assert!(first.path().join("nested/empty").is_dir());
    drop(second);
    std::fs::write(root.join("first.txt"), b"changed").unwrap();
    assert_eq!(
        std::fs::read(first.path().join("first.txt")).unwrap(),
        b"first"
    );
    let retained = first
        .retain(&store, &refs, "tree-example", &scope(), deadline())
        .unwrap();
    assert!(
        super::super::load(
            retained.directory(),
            &store,
            &refs,
            "tree-example",
            &scope(),
            deadline()
        )
        .is_ok()
    );
    let file = t.path().join("plain.bin");
    std::fs::write(&file, b"plain").unwrap();
    let plain = super::super::prepare(&file, Kind::File, &store, deadline()).unwrap();
    drop(plain);
    let result = cache::prune(
        &store,
        &refs,
        None,
        SystemTime::now() + Duration::from_secs(2 * 24 * 60 * 60),
        deadline(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["data"]["removedFiles"],
        0
    );
    assert!(retained.path().exists());
    std::fs::write(retained.path().join("nested/second.txt"), b"tampered").unwrap();
    assert!(
        super::super::load(
            retained.directory(),
            &store,
            &refs,
            "tree-example",
            &scope(),
            deadline()
        )
        .is_err()
    );
    let changed = prepare(&root, &store, &instance(), deadline()).unwrap();
    assert_ne!(changed.fingerprint().sha256, retained.fingerprint().sha256);
}
#[test]
fn invalid_directory_trees_fail_before_cache_creation() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::new(t.path().join("state"));
    let root = t.path().join("input");
    std::fs::create_dir(&root).unwrap();
    assert!(prepare(&root, &store, &instance(), deadline()).is_err());
    for index in 0..257 {
        std::fs::write(root.join(format!("{index}.txt")), b"x").unwrap();
    }
    assert!(prepare(&root, &store, &instance(), deadline()).is_err());
    assert!(!store.root().exists());
    let deep = t.path().join("deep");
    let mut leaf = deep.clone();
    for _ in 0..17 {
        leaf.push("next");
    }
    std::fs::create_dir_all(&leaf).unwrap();
    std::fs::write(leaf.join("data"), b"x").unwrap();
    assert!(prepare(&deep, &store, &instance(), deadline()).is_err());
    assert!(!store.root().exists());
    let linked = t.path().join("linked");
    std::fs::create_dir(&linked).unwrap();
    #[cfg(windows)]
    let created =
        std::os::windows::fs::symlink_file(root.join("0.txt"), linked.join("link")).is_ok();
    #[cfg(unix)]
    let created = std::os::unix::fs::symlink(root.join("0.txt"), linked.join("link")).is_ok();
    #[cfg(not(any(windows, unix)))]
    let created = false;
    if created {
        assert!(prepare(&linked, &store, &instance(), deadline()).is_err());
    }
}

#[test]
fn directory_uploads_cannot_snapshot_their_own_state() {
    let t = tempfile::tempdir().unwrap();
    let store = Store::new(t.path().join("state"));
    std::fs::write(t.path().join("data"), b"content").unwrap();
    assert_eq!(
        prepare(t.path(), &store, &instance(), deadline())
            .err()
            .unwrap()
            .code,
        "DIRECTORY_STATE_OVERLAP"
    );
    assert!(!store.root().exists());
    std::fs::create_dir_all(store.root().join("input")).unwrap();
    std::fs::write(store.root().join("input/data"), b"content").unwrap();
    assert_eq!(
        prepare(&store.root().join("input"), &store, &instance(), deadline())
            .err()
            .unwrap()
            .code,
        "DIRECTORY_STATE_OVERLAP"
    );
}

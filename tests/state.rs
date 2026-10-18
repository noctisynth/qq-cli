use std::fs;
use std::time::{Duration, Instant};

use qq_cli::config::Settings;
use qq_cli::state::{Instance, Registry, Store, select};

fn instance(id: &str, state: &str) -> Instance {
    Instance {
        id: id.into(),
        installation_id: "installation-test".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic-reference".into(),
        account: Some("10001".into()),
        run_id: "run-test".into(),
        state: state.into(),
        process_id: None,
        process_started: None,
    }
}

#[test]
fn login_confirmation_requires_matching_snapshot_and_never_promotes_offline_identity() {
    use qq_cli::{client::SessionInfo, session::confirm_login, state::Installation};
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let selected = instance("original-login", "login_required");
    let installed = Installation {
        id: selected.installation_id.clone(),
        version: "v4.18.38".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("original-artifact"),
    };
    let mut tx = store
        .transaction(Instant::now() + Duration::from_secs(1))
        .unwrap();
    tx.registry.instances.push(selected.clone());
    tx.registry.installations.push(installed.clone());
    tx.commit().unwrap();
    let deadline = || Instant::now() + Duration::from_secs(1);
    let mut info = SessionInfo {
        account: "10002".into(),
        nickname: None,
        online: false,
        version: "4.18.38".into(),
    };
    assert!(confirm_login(&store, &selected, &installed, &info, deadline()).unwrap());
    let observed = store.read().unwrap().instances[0].clone();
    assert_eq!(observed.state, "login_required");
    assert_eq!(observed.account.as_deref(), Some("10002"));
    info.online = true;
    assert_eq!(
        confirm_login(&store, &selected, &installed, &info, deadline())
            .unwrap_err()
            .code,
        "INSTANCE_CHANGED"
    );
    assert_eq!(store.read().unwrap().instances[0], observed);
    info.version = "4.18.39".into();
    assert_eq!(
        confirm_login(&store, &observed, &installed, &info, deadline())
            .unwrap_err()
            .code,
        "VERSION_MISMATCH"
    );
    info.version = "4.18.38".into();
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations[0].digest = "b".repeat(64);
    tx.commit().unwrap();
    assert_eq!(
        confirm_login(&store, &observed, &installed, &info, deadline())
            .unwrap_err()
            .code,
        "INSTANCE_CHANGED"
    );
    assert_eq!(store.read().unwrap().instances[0], observed);
}

#[test]
fn selection_never_guesses_from_readiness_or_array_order() {
    let mut registry = Registry::default();
    assert_eq!(
        select(&registry, None).unwrap_err().code,
        "INSTANCE_NOT_FOUND"
    );
    registry.instances.push(instance("first", "stopped"));
    assert_eq!(select(&registry, None).unwrap().id, "first");
    registry.instances.push(instance("second", "ready"));
    assert_eq!(
        select(&registry, None).unwrap_err().code,
        "INSTANCE_AMBIGUOUS"
    );
    assert_eq!(select(&registry, Some("second")).unwrap().id, "second");
    assert_eq!(
        select(&registry, Some("absent")).unwrap_err().code,
        "INSTANCE_NOT_FOUND"
    );
}

#[test]
fn settings_are_explicit_and_relative_state_is_config_relative() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config.json");
    fs::write(&path, r#"{"schemaVersion":1,"stateDir":"private-state"}"#).unwrap();
    let settings = Settings::load(Some(&path)).unwrap();
    assert_eq!(settings.state_dir, temp.path().join("private-state"));
    assert!(
        !settings.state_dir.exists(),
        "reading config must not create state"
    );
    for bad in [
        r#"{"schemaVersion":2}"#,
        r#"{"requestTimeoutSeconds":0}"#,
        r#"{"token":"synthetic-secret"}"#,
        "not-json",
    ] {
        fs::write(&path, bad).unwrap();
        assert!(Settings::load(Some(&path)).is_err());
    }
}

#[test]
fn explicit_status_acknowledges_changed_identity_but_refuses_concurrent_overwrite() {
    use qq_cli::session::{acknowledge_identity, remember_identity};
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let deadline = || Instant::now() + Duration::from_secs(3);
    let original = instance("first", "ready");
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.instances.push(original.clone());
    transaction.commit().unwrap();
    assert_eq!(
        remember_identity(&store, &original, "10002", deadline())
            .unwrap_err()
            .code,
        "ACCOUNT_CHANGED"
    );
    acknowledge_identity(&store, &original, "10002", deadline()).unwrap();
    assert_eq!(
        store.read().unwrap().instances[0].account.as_deref(),
        Some("10002")
    );
    assert_eq!(
        acknowledge_identity(&store, &original, "10003", deadline())
            .unwrap_err()
            .code,
        "ACCOUNT_CHANGED"
    );
}

#[test]
fn committed_state_is_atomic_and_crashed_transactions_do_not_commit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let store = Store::new(root.clone());
    assert!(store.read().unwrap().instances.is_empty());
    assert!(!root.exists());
    let mut transaction = store
        .transaction(Instant::now() + Duration::from_secs(3))
        .unwrap();
    transaction
        .registry
        .instances
        .push(instance("first", "ready"));
    assert!(store.read().unwrap().instances.is_empty());
    transaction.commit().unwrap();
    let mut transaction = store
        .transaction(Instant::now() + Duration::from_secs(3))
        .unwrap();
    transaction
        .registry
        .instances
        .push(instance("second", "ready"));
    drop(transaction);
    assert_eq!(store.read().unwrap().instances.len(), 1);
    let mut transaction = store
        .transaction(Instant::now() + Duration::from_secs(3))
        .unwrap();
    transaction.registry.instances[0].state = "stopped".into();
    transaction.commit().unwrap();
    assert_eq!(store.read().unwrap().instances[0].state, "stopped");
    fs::write(root.join("registry.json"), b"corrupt state").unwrap();
    assert!(
        store.read().is_err(),
        "corrupt state cannot be reinitialized"
    );
}

#[test]
fn state_locks_have_deadlines_and_release_on_drop() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("private"));
    let lock = store
        .transaction(Instant::now() + Duration::from_secs(3))
        .unwrap();
    let result = store.transaction(Instant::now() + Duration::from_millis(30));
    assert!(matches!(result, Err(ref error) if error.code == "STATE_BUSY"));
    drop(result);
    drop(lock);
    assert!(
        store
            .transaction(Instant::now() + Duration::from_secs(1))
            .is_ok()
    );
}

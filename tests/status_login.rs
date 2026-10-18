#![cfg(windows)]
#[path = "common/login_fixture.rs"]
mod fixture;
#[path = "common/login_service.rs"]
mod service;
use fixture::{Fixture, wait};
use qq_cli::storage::credentials::Credentials;
#[test]
fn pending_status_observes_control_but_only_onebot_confirms_actual_account() {
    if service::child() {
        return;
    }
    let fixture =
        Fixture::new("pending_status_observes_control_but_only_onebot_confirms_actual_account");
    let before = std::fs::read(fixture.store.root().join("registry.json")).unwrap();
    for (mode, phase) in [("", "qrcode_scanned"), ("control-ready", "ready")] {
        fixture.mode(mode);
        let (code, value) = fixture.status();
        assert_eq!(code, 0, "{value}");
        assert_eq!(value["data"]["state"], "login_required");
        assert_eq!(value["data"]["identityVerified"], false);
        assert!(value["data"]["account"].is_null());
        assert!(value.get("meta").is_none());
        assert_eq!(value["data"]["loginObservation"]["phase"], phase);
        assert_eq!(
            std::fs::read(fixture.store.root().join("registry.json")).unwrap(),
            before
        );
    }
    assert_eq!(fixture.counts()["auth"], 2);
    assert_eq!(fixture.counts()["control"], 2);
    assert_eq!(fixture.counts()["api"], 0);
    fixture.mode("api-pending");
    std::fs::write(
        fixture.temp.path().join("api-ready"),
        b"original API enable",
    )
    .unwrap();
    wait(&fixture.temp.path().join("api-listening"));
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["loginObservation"]["phase"], "initializing");
    assert!(value["data"]["account"].is_null());
    assert_eq!(
        std::fs::read(fixture.store.root().join("registry.json")).unwrap(),
        before
    );
    fixture.mode("bad-version");
    let (code, error) = fixture.status();
    assert_eq!(code, 3, "{error}");
    assert_eq!(error["error"]["code"], "VERSION_MISMATCH");
    let calls = fixture.counts()["api"].as_u64().unwrap();
    fixture.mode("bad-version-pending");
    let (code, error) = fixture.status();
    assert_eq!(code, 3, "{error}");
    assert_eq!(error["error"]["code"], "VERSION_MISMATCH");
    assert_eq!(fixture.counts()["api"].as_u64().unwrap(), calls + 1);
    assert_eq!(fixture.counts()["auth"], 3);
    fixture.mode("api-malformed");
    let (code, error) = fixture.status();
    assert_eq!(code, 3, "{error}");
    assert_eq!(error["error"]["code"], "BACKEND_TRANSPORT");
    assert_eq!(fixture.counts()["auth"], 3);
    fixture.mode("api-bad-identity");
    let (code, error) = fixture.status();
    assert_eq!(code, 3, "{error}");
    assert_eq!(error["error"]["code"], "BACKEND_PROTOCOL");
    assert_eq!(fixture.counts()["auth"], 3);
    assert_eq!(
        std::fs::read(fixture.store.root().join("registry.json")).unwrap(),
        before
    );
    fixture.mode("");
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["state"], "ready");
    assert_eq!(value["meta"]["scope"]["account"], "10001");
    assert!(value["data"].get("loginObservation").is_none());
    let actual = fixture.store.read().unwrap();
    assert_eq!(actual.instances[0].state, "ready");
    assert_eq!(actual.instances[0].account.as_deref(), Some("10001"));
    assert_eq!(fixture.counts()["auth"], 3);
}

#[test]
fn control_failures_and_concurrent_run_change_are_not_unverified_successes() {
    if service::child() {
        return;
    }
    let fixture =
        Fixture::new("control_failures_and_concurrent_run_change_are_not_unverified_successes");
    for (mode, expected) in [
        ("2fa", "CONTROL_2FA_REQUIRED"),
        ("reject", "CONTROL_REJECTED"),
    ] {
        fixture.mode(mode);
        let (code, error) = fixture.status();
        assert_ne!(code, 0);
        assert_eq!(error["error"]["code"], expected);
        assert_eq!(fixture.counts()["control"], 0);
    }
    fixture.mode("race");
    let (code, error) = fixture.status();
    assert_eq!(code, 4, "{error}");
    assert_eq!(error["error"]["code"], "STATUS_INPUT_CHANGED");
    assert_eq!(fixture.store.read().unwrap().instances[0].account, None);
    assert_eq!(fixture.counts()["auth"], 3);
    assert_eq!(fixture.counts()["control"], 1);
}

#[test]
fn missing_original_control_material_is_not_repaired_or_sent_to_the_backend() {
    if service::child() {
        return;
    }
    let fixture =
        Fixture::new("missing_original_control_material_is_not_repaired_or_sent_to_the_backend");
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    fixture.mode("api-pending");
    std::fs::write(
        fixture.temp.path().join("api-ready"),
        b"original API enabled",
    )
    .unwrap();
    wait(&fixture.temp.path().join("api-listening"));
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    assert_eq!(fixture.counts()["auth"], 2);
    std::fs::write(fixture.temp.path().join("api-stop"), b"original API stop").unwrap();
    wait(&fixture.temp.path().join("api-stopped"));
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    assert!(value["data"]["account"].is_null());
    assert_eq!(fixture.counts()["auth"], 3);
    for directory in ["retentions", "runtime-preparations"] {
        let file = std::fs::read_dir(fixture.store.root().join(directory))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let original = std::fs::read(&file).unwrap();
        std::fs::remove_file(&file).unwrap();
        let (code, error) = fixture.status();
        assert_ne!(code, 0, "{error}");
        assert!(!file.exists());
        assert_eq!(fixture.counts()["auth"], 3);
        std::fs::write(&file, original).unwrap();
    }
    let key = &fixture.keys[1];
    let original = fixture.credentials.get(key).unwrap();
    fixture.credentials.delete(key).unwrap();
    let (code, error) = fixture.status();
    assert_ne!(code, 0, "{error}");
    assert!(fixture.credentials.get(key).is_err());
    assert_eq!(fixture.counts()["auth"], 3);
    fixture.credentials.set(key, &original).unwrap();
    let (code, value) = fixture.status();
    assert_eq!(code, 0, "{value}");
    assert_eq!(fixture.counts()["auth"], 4);
}

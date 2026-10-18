#![cfg(windows)]
#[path = "common/login_fixture.rs"]
mod fixture;
#[path = "common/login_service.rs"]
mod service;
use fixture::{Fixture, deadline};
use qq_cli::storage::credentials::Credentials;

#[test]
fn public_remove_requires_stop_preserves_secrets_and_repeats_without_the_installation() {
    if service::child() {
        return;
    }
    let fixture = Fixture::new(
        "public_remove_requires_stop_preserves_secrets_and_repeats_without_the_installation",
    );
    fixture.mode("");
    assert_eq!(fixture.status().0, 0);
    let counts = fixture.counts();
    let (code, value) = fixture.run("napcat.remove", &["original-instance"]);
    assert_eq!(code, 4, "{value}");
    assert_eq!(value["error"]["code"], "STOP_PROOF_REQUIRED");
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
    assert_eq!(fixture.run("napcat.stop", &[]).0, 0);
    let secrets = fixture
        .keys
        .iter()
        .map(|key| fixture.credentials.get(key).unwrap())
        .collect::<Vec<_>>();
    let marker = fixture
        .store
        .root()
        .join("original-unresolved-request.json");
    std::fs::write(&marker, b"preserve original intent").unwrap();
    let (code, value) = fixture.run("napcat.remove", &["original-instance"]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["state"], "removed");
    assert_eq!(value["data"]["reused"], false);
    assert!(value.get("meta").is_none());
    assert!(fixture.store.read().unwrap().instances.is_empty());
    assert_eq!(fixture.store.read().unwrap().installations.len(), 1);
    assert_eq!(std::fs::read(marker).unwrap(), b"preserve original intent");
    for (key, secret) in fixture.keys.iter().zip(secrets) {
        assert_eq!(fixture.credentials.get(key).unwrap(), secret);
    }
    assert_eq!(fixture.counts(), counts);
    assert!(fixture.store.root().join("runtimes").exists());
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.installations.clear();
    tx.commit().unwrap();
    let (code, value) = fixture.run("napcat.remove", &["original-instance"]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["reused"], true);
}

#[test]
fn concurrent_public_removals_reuse_one_intent_and_unsigned_stopped_text_is_rejected() {
    if service::child() {
        return;
    }
    let fixture = Fixture::new(
        "concurrent_public_removals_reuse_one_intent_and_unsigned_stopped_text_is_rejected",
    );
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "stopped".into();
    tx.commit().unwrap();
    let (code, value) = fixture.run("napcat.remove", &["original-instance"]);
    assert_eq!(code, 4, "{value}");
    assert_eq!(value["error"]["code"], "STOP_PROOF_REQUIRED");
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
    assert!(!fixture.store.root().join("instance-removals").exists());
    assert_eq!(fixture.run("napcat.stop", &[]).0, 0);
    let first = fixture.spawn("napcat.remove", &["original-instance"]);
    let second = fixture.spawn("napcat.remove", &["original-instance"]);
    let (first_code, first) = fixture.finish("napcat.remove", first);
    let (second_code, second) = fixture.finish("napcat.remove", second);
    assert_eq!(first_code, 0, "{first}");
    assert_eq!(second_code, 0, "{second}");
    assert_ne!(first["data"]["reused"], second["data"]["reused"]);
    assert!(fixture.store.read().unwrap().instances.is_empty());
}

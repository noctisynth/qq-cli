#![cfg(windows)]
#[path = "common/login_fixture.rs"]
mod fixture;
#[path = "common/login_service.rs"]
mod service;
use fixture::{Fixture, deadline};
use qq_cli::storage::credentials::Credentials;

#[test]
fn public_stop_preserves_state_and_credentials_and_leaves_another_instance_alive() {
    if service::child() {
        return;
    }
    let name = "public_stop_preserves_state_and_credentials_and_leaves_another_instance_alive";
    let fixture = Fixture::new(name);
    let other = Fixture::new(name);
    fixture.mode("");
    assert_eq!(fixture.status().0, 0);
    let calls = fixture.counts();
    let secrets = fixture
        .keys
        .iter()
        .map(|key| fixture.credentials.get(key).unwrap())
        .collect::<Vec<_>>();
    let old = fixture.store.read().unwrap();
    let marker = fixture.store.root().join("original-preserved-request.json");
    std::fs::write(&marker, b"original unresolved write evidence").unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["state"], "stopped");
    assert_eq!(value["data"]["reused"], false);
    assert!(value.get("meta").is_none());
    assert_eq!(fixture.counts(), calls);
    let current = fixture.store.read().unwrap();
    assert_eq!(current.installations, old.installations);
    let mut expected = old.instances[0].clone();
    expected.state = "stopped".into();
    assert_eq!(current.instances[0], expected);
    assert_eq!(
        std::fs::read(&marker).unwrap(),
        b"original unresolved write evidence"
    );
    for (key, original) in fixture.keys.iter().zip(secrets) {
        assert_eq!(fixture.credentials.get(key).unwrap(), original);
    }
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["data"]["reused"], true);
    assert_eq!(fixture.status().1["data"]["state"], "stopped");
    other.process.as_ref().unwrap().verify(deadline()).unwrap();
    assert_eq!(other.status().0, 0);
}

#[test]
fn missing_dispatch_proof_or_signing_key_never_stops_the_running_process() {
    if service::child() {
        return;
    }
    let fixture =
        Fixture::new("missing_dispatch_proof_or_signing_key_never_stops_the_running_process");
    let file = std::fs::read_dir(fixture.store.root().join("run-jobs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let original = std::fs::read(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 4, "{value}");
    assert_eq!(value["error"]["code"], "STOP_UNVERIFIABLE");
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
    assert!(!fixture.store.root().join("instance-stops").exists());
    std::fs::write(&file, original).unwrap();
    let original = fixture.credentials.get("reference-signing-key").unwrap();
    fixture.credentials.delete("reference-signing-key").unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_ne!(code, 0, "{value}");
    assert_eq!(value["error"]["code"], "CREDENTIAL_NOT_FOUND");
    assert!(fixture.credentials.get("reference-signing-key").is_err());
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
    assert!(!fixture.store.root().join("instance-stops").exists());
    fixture
        .credentials
        .set("reference-signing-key", &original)
        .unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 0, "{value}");
}

#[test]
fn ambiguous_stop_does_not_access_secrets_and_explicit_selection_still_works() {
    if service::child() {
        return;
    }
    let fixture =
        Fixture::new("ambiguous_stop_does_not_access_secrets_and_explicit_selection_still_works");
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    let mut another = tx.registry.instances[0].clone();
    another.id = "another-instance".into();
    tx.registry.instances.push(another);
    tx.commit().unwrap();
    let signing_key = fixture.credentials.get("reference-signing-key").unwrap();
    fixture.credentials.delete("reference-signing-key").unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 2, "{value}");
    assert_eq!(value["error"]["code"], "INSTANCE_AMBIGUOUS");
    let (code, value) = fixture.run("napcat.stop", &["--instance", "missing"]);
    assert_eq!(code, 4, "{value}");
    assert_eq!(value["error"]["code"], "INSTANCE_NOT_FOUND");
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
    fixture
        .credentials
        .set("reference-signing-key", &signing_key)
        .unwrap();
    let (code, value) = fixture.run("napcat.stop", &["--instance", "original-instance"]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(
        fixture.store.read().unwrap().instances[1].state,
        "login_required"
    );
}

#[test]
fn concurrent_stop_calls_share_the_same_completion_proof() {
    if service::child() {
        return;
    }
    let fixture = Fixture::new("concurrent_stop_calls_share_the_same_completion_proof");
    let first = fixture.spawn("napcat.stop", &[]);
    let second = fixture.spawn("napcat.stop", &[]);
    let (first_code, first) = fixture.finish("napcat.stop", first);
    let (second_code, second) = fixture.finish("napcat.stop", second);
    assert_eq!(first_code, 0, "{first}");
    assert_eq!(second_code, 0, "{second}");
    assert_ne!(first["data"]["reused"], second["data"]["reused"]);
    assert_eq!(fixture.store.read().unwrap().instances[0].state, "stopped");
}

#[test]
fn unavailable_original_job_with_pending_stop_remains_unknown_without_stopping_a_replacement() {
    if service::child() {
        return;
    }
    use qq_cli::{
        native::job::Job,
        storage::{config::read_json, references::References, state::atomic_json},
    };
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    let fixture = Fixture::new(
        "unavailable_original_job_with_pending_stop_remains_unknown_without_stopping_a_replacement",
    );
    let refs = References::existing(&fixture.store, &fixture.credentials).unwrap();
    let file = std::fs::read_dir(fixture.store.root().join("run-jobs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let pointer: Value = read_json(&file, 4096).unwrap();
    let mut record: Value = refs
        .get("j1", pointer["reference"].as_str().unwrap())
        .unwrap();
    let empty = Job::create(deadline()).unwrap();
    let disappeared = serde_json::to_value(empty.id()).unwrap();
    drop(empty);
    record["job"] = disappeared.clone();
    atomic_json(
        &file,
        &json!({"schemaVersion":1,"reference":refs.issue("j1", &record).unwrap()}),
    )
    .unwrap();
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "stopping".into();
    let instance = tx.registry.instances[0].clone();
    tx.commit().unwrap();
    let target = json!({"instanceId":instance.id,"runId":instance.run_id,"installationId":instance.installation_id,"installationDigest":"a".repeat(64)});
    let reference = refs
        .issue(
            "p1",
            &json!({"schemaVersion":1,"target":target,"job":disappeared,"outcome":"pending"}),
        )
        .unwrap();
    let directory = fixture.store.root().join("instance-stops");
    qq_cli::storage::state::private_dir(&directory).unwrap();
    let digest = Sha256::digest(serde_json::to_vec(&(&instance.id, &instance.run_id)).unwrap());
    atomic_json(
        &directory.join(format!("{digest:x}.json")),
        &json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    let (code, value) = fixture.run("napcat.stop", &[]);
    assert_eq!(code, 5, "{value}");
    assert_eq!(value["error"]["code"], "STOP_OUTCOME_UNKNOWN");
    assert_eq!(value["error"]["outcome"], "unknown");
    assert_eq!(value["error"]["nextAction"]["command"], "doctor");
    assert_eq!(fixture.store.read().unwrap().instances[0].state, "stopping");
    fixture
        .process
        .as_ref()
        .unwrap()
        .verify(deadline())
        .unwrap();
}

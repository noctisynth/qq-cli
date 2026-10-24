use qq_cli::{
    core::error::{AppError, Result},
    napcat::runtime::start_intent::{self, Begin, Target},
    storage::credentials::Credentials,
    storage::references::References,
    storage::state::{Installation, Instance, Store},
};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn until() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
struct NoCredentials;
impl Credentials for NoCredentials {
    fn get(&self, _: &str) -> Result<String> {
        panic!("no credential access for missing/message/conflicting requests")
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("read-only query")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("read-only query")
    }
}
struct Signer;
impl Credentials for Signer {
    fn get(&self, key: &str) -> Result<String> {
        assert_eq!(key, "reference-signing-key");
        use base64::Engine;
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([31; 32]))
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("read-only query")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("read-only query")
    }
}
fn setup(store: &Store) -> Target {
    let target = Target {
        instance_id: "original-instance".into(),
        run_id: "original-run".into(),
        installation_id: "original-installation".into(),
        installation_digest: "a".repeat(64),
        input_hash: "b".repeat(64),
    };
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.installations.push(Installation {
        id: target.installation_id.clone(),
        version: "v4.18.38".into(),
        digest: target.installation_digest.clone(),
        directory: store.root().join("unused-original-installation"),
    });
    tx.registry.instances.push(Instance {
        id: target.instance_id.clone(),
        run_id: target.run_id.clone(),
        installation_id: target.installation_id.clone(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "unused-original-key".into(),
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    target
}
fn value(envelope: qq_cli::core::output::Envelope) -> Value {
    serde_json::to_value(envelope).unwrap()
}
fn validate(value: &Value) {
    let schema = qq_cli::cli::registry::get("request.get").unwrap();
    let validator = jsonschema::validator_for(&schema["responseSchema"]).unwrap();
    assert!(validator.is_valid(value), "{value}");
}

#[test]
fn missing_and_message_queries_never_open_signing_credentials_or_initialize_missing_state() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    assert_eq!(
        qq_cli::requests::query::get(&store, "original-missing", None, &NoCredentials, until())
            .err()
            .unwrap()
            .code,
        "REQUEST_NOT_FOUND"
    );
    assert!(!store.root().exists());
    let scope = qq_cli::core::domain::Scope {
        instance_id: "original-instance".into(),
        account: "10001".into(),
    };
    let qq_cli::requests::ledger::Begin::Fresh(intent) = qq_cli::requests::ledger::begin(
        &store,
        "original-message",
        "send",
        &scope,
        &json!({}),
        until(),
    )
    .unwrap() else {
        panic!("fresh message")
    };
    intent
        .finish(
            qq_cli::requests::ledger::Outcome::Accepted,
            Some(json!({"messageId":"1","conversation":"private:10002"})),
        )
        .unwrap();
    let result = value(
        qq_cli::requests::query::get(&store, "original-message", None, &NoCredentials, until())
            .unwrap(),
    );
    validate(&result);
    assert_eq!(result["data"]["outcome"], "accepted");
    assert_eq!(result["meta"]["scope"]["account"], "10001");
    assert!(!store.root().join("references").exists());
}

#[test]
fn signed_start_query_reports_durable_outcomes_without_mutation_or_live_identity_claims() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let target = setup(&store);
    let refs = References::with_key(store.root().join("references"), [31; 32]);
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "original-start", &target, until()).unwrap()
    else {
        panic!("fresh start")
    };
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    let pending = value(
        qq_cli::requests::query::get(&store, "original-start", None, &Signer, until()).unwrap(),
    );
    validate(&pending);
    assert_eq!(pending["data"]["outcome"], "pending");
    assert_eq!(
        start_intent::get(&store, &refs, "original-start")
            .unwrap()
            .outcome,
        start_intent::Outcome::Pending
    );
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    intent.dispatch(|_| Ok(())).unwrap(); // Original no-op, not native dispatch.
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.commit().unwrap();
    let result = value(
        qq_cli::requests::query::get(&store, "original-start", None, &Signer, until()).unwrap(),
    );
    validate(&result);
    assert_eq!(result["data"]["outcome"], "dispatched");
    assert_eq!(result["data"]["instanceState"], "ready");
    assert_eq!(result["data"]["identityVerified"], false);
    assert!(result.get("meta").is_none());
    assert_eq!(
        qq_cli::requests::query::get(
            &store,
            "original-start",
            Some("other-instance"),
            &Signer,
            until()
        )
        .err()
        .unwrap()
        .code,
        "INSTANCE_CONFLICT"
    );
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.instances[0].run_id = "replacement-run".into();
    tx.commit().unwrap();
    let result = value(
        qq_cli::requests::query::get(&store, "original-start", None, &Signer, until()).unwrap(),
    );
    validate(&result);
    assert_eq!(result["data"]["runId"], target.run_id);
    assert_eq!(result["data"]["instanceState"], "changed");
    assert_eq!(result["data"]["nextAction"]["command"], "napcat.instances");
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.instances.clear();
    tx.commit().unwrap();
    let result = value(
        qq_cli::requests::query::get(&store, "original-start", None, &Signer, until()).unwrap(),
    );
    validate(&result);
    assert_eq!(result["data"]["instanceState"], "unregistered");
    assert!(!store.root().join("run-jobs").exists());
}

#[test]
fn request_families_cannot_reuse_keys_and_legacy_collisions_are_not_selected() {
    for message_first in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let target = setup(&store);
        let refs = References::with_key(store.root().join("references"), [31; 32]);
        let scope = qq_cli::core::domain::Scope {
            instance_id: target.instance_id.clone(),
            account: "10001".into(),
        };
        if message_first {
            let qq_cli::requests::ledger::Begin::Fresh(intent) = qq_cli::requests::ledger::begin(
                &store,
                "shared-key",
                "send",
                &scope,
                &json!({}),
                until(),
            )
            .unwrap() else {
                panic!("message")
            };
            intent
                .finish(qq_cli::requests::ledger::Outcome::Unknown, None)
                .unwrap();
            assert_eq!(
                start_intent::begin(&store, &refs, "shared-key", &target, until())
                    .err()
                    .unwrap()
                    .code,
                "REQUEST_CONFLICT"
            );
            assert!(!store.root().join("start-requests").exists());
        } else {
            let Begin::Fresh(intent) =
                start_intent::begin(&store, &refs, "shared-key", &target, until()).unwrap()
            else {
                panic!("start")
            };
            drop(intent);
            assert_eq!(
                qq_cli::requests::ledger::begin(
                    &store,
                    "shared-key",
                    "send",
                    &scope,
                    &json!({}),
                    until()
                )
                .err()
                .unwrap()
                .code,
                "REQUEST_CONFLICT"
            );
            assert!(!store.root().join("requests").exists());
            // Simulate an old pair of independent records without using the guarded writer.
            use sha2::{Digest, Sha256};
            let directory = store.root().join("requests");
            qq_cli::storage::state::private_dir(&directory).unwrap();
            qq_cli::storage::state::atomic_json(
                &directory.join(format!("{:x}.json", Sha256::digest(b"shared-key"))),
                &qq_cli::requests::ledger::Record {
                    schema_version: 1,
                    request_id: "shared-key".into(),
                    operation: "send".into(),
                    scope,
                    payload_hash: "c".repeat(64),
                    outcome: qq_cli::requests::ledger::Outcome::Unknown,
                    receipt: None,
                },
            )
            .unwrap();
            assert_eq!(
                qq_cli::requests::query::get(&store, "shared-key", None, &NoCredentials, until())
                    .err()
                    .unwrap()
                    .code,
                "REQUEST_CONFLICT"
            );
        }
    }
}

#[test]
fn concurrent_message_and_start_intents_cannot_both_claim_the_same_key() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let target = setup(&store);
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let message = scope.spawn(|| {
            barrier.wait();
            match qq_cli::requests::ledger::begin(
                &store,
                "concurrent-key",
                "send",
                &qq_cli::core::domain::Scope {
                    instance_id: target.instance_id.clone(),
                    account: "10001".into(),
                },
                &json!({}),
                until(),
            ) {
                Ok(qq_cli::requests::ledger::Begin::Fresh(intent)) => {
                    intent
                        .finish(qq_cli::requests::ledger::Outcome::Unknown, None)
                        .unwrap();
                    "claimed"
                }
                Err(error) => error.code,
                _ => panic!("new namespace"),
            }
        });
        let start = scope.spawn(|| {
            let refs = References::with_key(store.root().join("references"), [31; 32]);
            barrier.wait();
            match start_intent::begin(&store, &refs, "concurrent-key", &target, until()) {
                Ok(Begin::Fresh(intent)) => {
                    drop(intent);
                    "claimed"
                }
                Err(error) => error.code,
                _ => panic!("new namespace"),
            }
        });
        [message.join().unwrap(), start.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| **r == "claimed").count(), 1);
    assert_eq!(
        results.iter().filter(|r| **r == "REQUEST_CONFLICT").count(),
        1
    );
    assert!(!store.root().join("run-jobs").exists());
}

#[test]
fn missing_signing_key_is_never_provisioned_by_start_query() {
    struct Missing;
    impl Credentials for Missing {
        fn get(&self, _: &str) -> Result<String> {
            Err(AppError::new(
                "CREDENTIAL_NOT_FOUND",
                "Original test key missing.",
                4,
            ))
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            panic!("query cannot provision a key")
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("query cannot delete a key")
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let target = setup(&store);
    let refs = References::with_key(store.root().join("references"), [31; 32]);
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "original-key", &target, until()).unwrap()
    else {
        panic!("start")
    };
    drop(intent);
    assert_eq!(
        qq_cli::requests::query::get(&store, "original-key", None, &Missing, until())
            .err()
            .unwrap()
            .code,
        "CREDENTIAL_NOT_FOUND"
    );
    assert_eq!(
        start_intent::get(&store, &refs, "original-key")
            .unwrap()
            .outcome,
        start_intent::Outcome::Pending
    );
}

#[cfg(windows)]
#[test]
fn public_request_query_uses_only_the_original_temporary_vault_signer() {
    use clap::Parser;
    use qq_cli::storage::credentials::SystemCredentials;
    struct Cleanup(SystemCredentials);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let target = setup(&store);
    let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
    assert_eq!(
        credentials
            .0
            .get("reference-signing-key")
            .err()
            .unwrap()
            .code,
        "CREDENTIAL_NOT_FOUND"
    );
    let refs = References::open(&store, &credentials.0, until()).unwrap();
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "original-cli-request", &target, until()).unwrap()
    else {
        panic!("start")
    };
    drop(intent); // Pending is preserved by the read-only public query.
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    let config = temp.path().join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&qq_cli::storage::config::Config {
            state_dir: Some(store.root().to_owned()),
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    let cli = qq_cli::cli::args::Cli::try_parse_from([
        std::ffi::OsStr::new("qq-cli"),
        std::ffi::OsStr::new("--config"),
        config.as_os_str(),
        std::ffi::OsStr::new("request"),
        std::ffi::OsStr::new("get"),
        std::ffi::OsStr::new("original-cli-request"),
    ])
    .unwrap();
    let result = qq_cli::app::execute(cli);
    assert_eq!(result.exit_code(), 0);
    let result = value(result);
    validate(&result);
    assert_eq!(result["data"]["outcome"], "pending");
    assert_eq!(result["data"]["instanceId"], target.instance_id);
    assert!(result.get("meta").is_none());
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    assert_eq!(
        start_intent::get(&store, &refs, "original-cli-request")
            .unwrap()
            .outcome,
        start_intent::Outcome::Pending
    );
    credentials.0.delete("reference-signing-key").unwrap();
    assert_eq!(
        qq_cli::requests::query::get(
            &store,
            "original-cli-request",
            None,
            &credentials.0,
            until()
        )
        .err()
        .unwrap()
        .code,
        "CREDENTIAL_NOT_FOUND"
    );
    assert_eq!(
        credentials
            .0
            .get("reference-signing-key")
            .err()
            .unwrap()
            .code,
        "CREDENTIAL_NOT_FOUND"
    );
}

#[test]
fn macos_creation_checkpoint_can_be_queried_offline_without_migration_or_dispatch() {
    use qq_cli::{
        napcat::runtime::{runtime_config::Ports, runtime_store::Plan},
        storage::state::{atomic_json, private_dir},
    };
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let target = setup(&store);
    let refs = References::with_key(store.root().join("references"), [31; 32]);
    let installed = store.read().unwrap().installations[0].clone();
    let plan = Plan {
        instance_id: target.instance_id.clone(),
        run_id: target.run_id.clone(),
        version: installed.version.clone(),
        ports: Ports {
            http: 24001,
            events: 24002,
            control: 24003,
        },
    };
    let reference = refs.issue("n2", &json!({"schemaVersion":1,"requestId":"mac-start","installation":installed,"plan":plan,"phase":"registered"})).unwrap();
    let directory = store.root().join("instance-creations");
    private_dir(&directory).unwrap();
    let pointer = directory.join(format!("{:x}.json", Sha256::digest(b"mac-start")));
    atomic_json(&pointer, &json!({"schemaVersion":1,"reference":reference})).unwrap();
    let before = std::fs::read(&pointer).unwrap();
    let response =
        value(qq_cli::requests::query::get(&store, "mac-start", None, &Signer, until()).unwrap());
    validate(&response);
    assert_eq!(response["data"]["operation"], "napcat.start");
    assert_eq!(response["data"]["outcome"], "pending");
    assert_eq!(response["data"]["identityVerified"], false);
    assert_eq!(std::fs::read(pointer).unwrap(), before);
    assert!(!store.root().join("start-requests").exists());
}

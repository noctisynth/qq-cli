//! Contract tests exercise actual production serializers, not schema-only replicas.
use qq_cli::core::{
    contracts::{
        installation::{Installation, Installed, RecordedInstance, Uninstalled},
        scalar::{LocalId, No, Yes},
    },
    error::AppError,
    output::Envelope,
};
use serde_json::{Value, json};
fn validator(command: &str) -> jsonschema::Validator {
    jsonschema::validator_for(&qq_cli::cli::registry::get(command).unwrap()["responseSchema"])
        .unwrap()
}
#[test]
fn actual_installation_payloads_and_error_envelopes_match_their_derived_schemas() {
    let items = [
        (
            "napcat.install",
            Envelope::typed(Installed {
                installation_id: LocalId::new("install-example").unwrap(),
                version: "v4.18.38".into(),
                cached: false,
            }),
        ),
        (
            "napcat.uninstall",
            Envelope::typed(Uninstalled {
                installation_id: LocalId::new("install-example").unwrap(),
                removed: Yes,
            }),
        ),
        (
            "napcat.installations",
            Envelope::typed(vec![Installation {
                installation_id: LocalId::new("install-example").unwrap(),
                version: "v4.18.38".into(),
                references: 2,
            }]),
        ),
        (
            "napcat.instances",
            Envelope::typed(vec![RecordedInstance {
                instance_id: LocalId::new("instance-example").unwrap(),
                installation_id: LocalId::new("install-example").unwrap(),
                recorded_state: "stopped".into(),
                identity_verified: No,
            }]),
        ),
    ];
    for (name, envelope) in items {
        let value = serde_json::to_value(envelope).unwrap();
        let validator = validator(name);
        assert!(validator.is_valid(&value), "{name}: {value}");
        let error = serde_json::to_value(Envelope::failure(AppError::new(
            "SYNTHETIC_ERROR",
            "Synthetic contract failure.",
            4,
        )))
        .unwrap();
        assert!(validator.is_valid(&error));
        for mutate in 0..5 {
            let mut bad = value.clone();
            match mutate {
                0 => bad["schemaVersion"] = json!(2),
                1 => bad["ok"] = json!(false),
                2 => bad["extra"] = json!("never public"),
                3 => {
                    bad.as_object_mut().unwrap().remove("data");
                }
                _ => {
                    bad["meta"] = Value::Null;
                }
            }
            assert!(
                !validator.is_valid(&bad),
                "{name} accepted invalid envelope {bad}"
            );
        }
        let mut bad = value;
        let data = if bad["data"].is_array() {
            &mut bad["data"][0]
        } else {
            &mut bad["data"]
        };
        data["unexpected"] = json!("never public");
        assert!(!validator.is_valid(&bad));
    }
}
#[test]
fn literal_markers_and_local_id_constraints_cannot_drift() {
    let schema = validator("napcat.uninstall");
    for invalid in [
        json!({"installationId":"install-example","removed":false}),
        json!({"installationId":123,"removed":true}),
        json!({"installationId":"../unsafe","removed":true}),
    ] {
        assert!(!schema.is_valid(&json!({"schemaVersion":1,"ok":true,"data":invalid})));
    }
    assert!(LocalId::new("").is_err());
    assert!(LocalId::new("../unsafe").is_err());
    assert!(LocalId::new(&"a".repeat(129)).is_err());
    let schema = validator("napcat.instances");
    assert!(!schema.is_valid(&json!({"schemaVersion":1,"ok":true,"data":[{"instanceId":"instance-example","installationId":"install-example","recordedState":"ready","identityVerified":true}]})));
}
#[test]
fn list_commands_serialize_actual_typed_records_offline() {
    use qq_cli::storage::state::{Installation as StoredInstallation, Instance, Store};
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let mut tx = store
        .transaction(std::time::Instant::now() + std::time::Duration::from_secs(5))
        .unwrap();
    tx.registry.installations.push(StoredInstallation {
        id: "install-example".into(),
        version: "v4.18.38".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("unused-artifact"),
    });
    tx.registry.instances.push(Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "not-needed".into(),
        account: None,
        run_id: "run-example".into(),
        state: "stopped".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let config = temp.path().join("config.json");
    std::fs::write(&config, json!({"stateDir":store.root()}).to_string()).unwrap();
    for name in ["installations", "instances"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(["--config", config.to_str().unwrap(), "napcat", name])
            .env_remove("QQ_CLI_CONFIG")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(output.stderr.is_empty());
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(validator(&format!("napcat.{name}")).is_valid(&value));
        assert_eq!(value["data"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn instance_wire_states_and_lifecycle_results_use_actual_derived_contracts() {
    use qq_cli::core::contracts::instance::*;
    let observed = || Observed {
        state: ObservedState::Ready,
        online: true,
        version: "v4.18.38".into(),
        nickname: Some("synthetic".into()),
    };
    let cases = [
        (
            "status",
            Envelope::typed(observed()).scoped(qq_cli::core::domain::Scope {
                instance_id: "example".into(),
                account: "10001".into(),
            }),
        ),
        (
            "status",
            Envelope::typed(Pending {
                state: PendingState::LoginRequired,
                account: (),
                identity_verified: No,
                login_observation: Some(Observation {
                    phase: Phase::QrcodeScanned,
                    is_login: false,
                    is_offline: false,
                    qr_login_accepted: true,
                    core_ready: false,
                }),
            }),
        ),
        (
            "napcat.start",
            Envelope::typed(Started {
                instance_id: LocalId::new("example").unwrap(),
                status: observed(),
                reused: false,
            })
            .scoped(qq_cli::core::domain::Scope {
                instance_id: "example".into(),
                account: "10001".into(),
            }),
        ),
        (
            "napcat.stop",
            Envelope::typed(Stopped {
                instance_id: LocalId::new("example").unwrap(),
                state: StoppedState::Stopped,
                reused: false,
            }),
        ),
        (
            "napcat.remove",
            Envelope::typed(Removed {
                instance_id: LocalId::new("example").unwrap(),
                state: RemovedState::Removed,
                reused: true,
            }),
        ),
    ];
    for (command, body) in cases {
        let value = serde_json::to_value(body).unwrap();
        let v = validator(command);
        assert!(
            v.is_valid(&value),
            "{command}: {:?}",
            v.iter_errors(&value)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
        );
        let mut bad = value.clone();
        bad["data"]["state"] = json!("invented");
        assert!(!v.is_valid(&bad));
    }
    let value = serde_json::to_value(Envelope::typed(observed())).unwrap();
    for name in ["napcat.restart", "napcat.update", "napcat.rollback"] {
        assert!(!validator(name).is_valid(&value));
    }
}

#[test]
fn offline_discovery_outputs_validate_against_their_own_actual_types() {
    let items = [
        (
            "schema.list",
            Envelope::typed(qq_cli::cli::registry::list()),
        ),
        (
            "schema.get",
            Envelope::typed(qq_cli::cli::registry::describe("schema.get").unwrap()),
        ),
        ("skill.list", Envelope::typed(qq_cli::cli::skill::list())),
        (
            "skill.get",
            Envelope::typed(qq_cli::cli::skill::get("events").unwrap()),
        ),
    ];
    for (command, envelope) in items {
        let value = serde_json::to_value(envelope).unwrap();
        let v = validator(command);
        assert!(v.is_valid(&value), "{command}: {value}");
        let mut bad = value.clone();
        if bad["data"].is_array() {
            bad["data"][0]["extra"] = json!(true);
        } else {
            bad["data"]["extra"] = json!(true);
        }
        assert!(!v.is_valid(&bad));
    }
}

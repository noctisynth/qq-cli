use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Instant;

use qq_cli::core::error::{AppError, Result};
use qq_cli::napcat::compatibility::{self, EvidenceKind, Key, Verdict, Witness};
use qq_cli::napcat::doctor::{self, Probe};
use qq_cli::napcat::install::catalog;
use qq_cli::napcat::qq::{self, Application};
use qq_cli::native::platform::Host;
use qq_cli::storage::config::Config;
use qq_cli::storage::state::{Installation, Instance, Registry, Store};
use qq_cli::transport::client::SessionInfo;
use serde_json::Value;

#[path = "common/qq_fixture.rs"]
mod fixture;

struct Fake {
    paths: Vec<PathBuf>,
    artifact_ok: bool,
    api_ok: bool,
    api_calls: Cell<usize>,
}
fn host() -> Host {
    Host {
        os: "windows".into(),
        os_build: Some(10000),
        native_arch: "x86_64".into(),
        process_arch: "x86_64".into(),
    }
}
impl Probe for Fake {
    fn host(&self) -> Host {
        host()
    }
    fn qq_paths(&self) -> Result<Vec<PathBuf>> {
        Ok(self.paths.clone())
    }
    fn inspect_qq(&self, path: &Path, deadline: Instant) -> Result<Application> {
        qq::inspect(path, deadline)
    }
    fn artifact(&self, _: &Store, _: &Installation, _: Instant) -> Result<()> {
        if self.artifact_ok {
            Ok(())
        } else {
            Err(AppError::new(
                "INSTALLATION_MODIFIED",
                "Synthetic integrity refusal",
                4,
            ))
        }
    }
    fn api(&self, _: &Store, _: &Config, _: &Instance, _: &str, _: Instant) -> Result<SessionInfo> {
        self.api_calls.set(self.api_calls.get() + 1);
        if !self.api_ok {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Synthetic identity refusal",
                4,
            ));
        }
        Ok(SessionInfo {
            account: "10001".into(),
            nickname: None,
            online: true,
            version: "4.18.38".into(),
        })
    }
}
fn fake(path: PathBuf) -> Fake {
    Fake {
        paths: vec![path],
        artifact_ok: true,
        api_ok: true,
        api_calls: Cell::new(0),
    }
}
fn registry(root: &Path) -> Registry {
    let release = catalog::lookup("v4.18.38").unwrap();
    Registry {
        schema_version: 1,
        maintenance_intents: Default::default(),
        installations: vec![Installation {
            id: "synthetic-installation".into(),
            version: release.version.into(),
            digest: release.archive.sha256.into(),
            directory: root.join("synthetic-installation"),
        }],
        instances: vec![],
    }
}
fn instance() -> Instance {
    Instance {
        id: "synthetic-instance".into(),
        installation_id: "synthetic-installation".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: Some("10001".into()),
        run_id: "synthetic-run".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}
fn key(path: &Path) -> Key {
    let release = catalog::lookup("v4.18.38").unwrap();
    let app = qq::inspect(path, Instant::now() + std::time::Duration::from_secs(5)).unwrap();
    Key::from_installation(host(), &release, &app).unwrap()
}

#[test]
fn exact_framework_evidence_is_required_and_conflicting_evidence_is_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(temp.path(), false);
    let key = key(&executable);
    assert_eq!(compatibility::evaluate(&key, &[]).state, Verdict::Unknown);
    for kind in [EvidenceKind::StaticInspection, EvidenceKind::ShellSmoke] {
        assert_eq!(
            compatibility::evaluate(
                &key,
                &[Witness {
                    key: key.clone(),
                    verdict: Verdict::Supported,
                    reference: "synthetic static observation".into(),
                    kind
                }]
            )
            .state,
            Verdict::Unknown
        );
    }
    let witnesses = vec![Witness {
        key: key.clone(),
        verdict: Verdict::Supported,
        reference: "synthetic framework trial".into(),
        kind: EvidenceKind::FrameworkStartup,
    }];
    assert_eq!(
        compatibility::evaluate(&key, &witnesses).state,
        Verdict::Supported
    );
    let mut changed = key.clone();
    changed.qq_build = "60002".into();
    assert_eq!(
        compatibility::evaluate(&changed, &witnesses).state,
        Verdict::Unknown
    );
    let mut changed = key.clone();
    changed.qq_installation_id = "different-synthetic-installation".into();
    assert_eq!(
        compatibility::evaluate(&changed, &witnesses).state,
        Verdict::Unknown
    );
    let mut changed = key.clone();
    changed.loader_sha256 = "f".repeat(64);
    assert_eq!(
        compatibility::evaluate(&changed, &witnesses).state,
        Verdict::Unknown
    );
    let mut changed = key.clone();
    changed.host.os_build = Some(10001);
    assert_eq!(
        compatibility::evaluate(&changed, &witnesses).state,
        Verdict::Unknown
    );
    let mut conflicting = witnesses;
    conflicting.push(Witness {
        key: key.clone(),
        verdict: Verdict::Unsupported,
        reference: "synthetic conflicting trial".into(),
        kind: EvidenceKind::FrameworkStartup,
    });
    assert_eq!(
        compatibility::evaluate(&key, &conflicting).reason,
        "EVIDENCE_CONFLICT"
    );
}

#[test]
fn diagnostic_collection_is_read_only_and_does_not_promote_api_success_to_compatibility() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(&temp.path().join("application"), false);
    let probe = fake(executable);
    let store = Store::new(temp.path().join("state"));
    let config = Config::default();
    let mut registry = registry(temp.path());
    registry.instances.push(instance());
    let expired = doctor::execute_until(
        &store,
        &config,
        &registry,
        None,
        None,
        &probe,
        Instant::now() - std::time::Duration::from_millis(1),
    );
    assert_eq!(expired.err().unwrap().code, "DEADLINE");
    assert_eq!(probe.api_calls.get(), 0);
    assert!(!store.root().exists());
    let result: Value = serde_json::to_value(
        doctor::execute(&store, &config, &registry, None, None, &probe).unwrap(),
    )
    .unwrap();
    assert_contract(&result);
    assert_eq!(result["data"]["canStart"], false);
    assert_eq!(result["meta"]["scope"]["account"], "10001");
    let checks = result["data"]["checks"].as_array().unwrap();
    assert_eq!(
        checks.iter().find(|check| check["name"] == "api").unwrap()["state"],
        "ready"
    );
    assert_eq!(
        checks
            .iter()
            .find(|check| check["name"] == "compatibility")
            .unwrap()["result"]["state"],
        "unknown"
    );
    assert!(!store.root().exists());
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains(temp.path().to_str().unwrap())
    );
}

#[test]
fn start_eligibility_requires_integrity_and_exact_evidence_not_license_annotation() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(&temp.path().join("application"), false);
    let key = key(&executable);
    let mut probe = fake(executable);
    let store = Store::new(temp.path().join("state"));
    let config = Config::default();
    let registry = registry(temp.path());
    let mut release = catalog::lookup("v4.18.38").unwrap();
    let witnesses = vec![Witness {
        key,
        verdict: Verdict::Supported,
        reference: "synthetic framework trial".into(),
        kind: EvidenceKind::FrameworkStartup,
    }];
    let run = |probe: &Fake, release: &catalog::Release| {
        let value = serde_json::to_value(
            doctor::diagnose(&store, &config, &registry, None, release, &witnesses, probe).unwrap(),
        )
        .unwrap();
        assert_contract(&value);
        value
    };
    assert_eq!(run(&probe, &release)["data"]["canStart"], true);
    let unconfirmed = run(&probe, &release);
    assert!(
        unconfirmed["data"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| {
                check["name"] == "authorization" && check["code"] == "LICENSE_REVIEW_PENDING"
            })
    );
    release.authorization_evidence = Some("synthetic test prerequisite only");
    assert_eq!(run(&probe, &release)["data"]["canStart"], true);
    probe.artifact_ok = false;
    assert_eq!(run(&probe, &release)["data"]["canStart"], false);
    assert!(!store.root().exists());
    assert_eq!(probe.api_calls.get(), 0);
}

#[test]
fn ambiguous_instances_or_qq_candidates_never_select_by_array_order() {
    let temp = tempfile::tempdir().unwrap();
    let first = fixture::fixture(&temp.path().join("first"), false);
    let second = fixture::fixture(&temp.path().join("second"), false);
    let mut probe = fake(first.clone());
    let store = Store::new(temp.path().join("state"));
    let config = Config::default();
    let mut registry = registry(temp.path());
    registry.instances.push(instance());
    let mut other = instance();
    other.id = "another-synthetic-instance".into();
    registry.instances.push(other);
    assert_eq!(
        doctor::execute(&store, &config, &registry, None, None, &probe)
            .err()
            .unwrap()
            .code,
        "INSTANCE_AMBIGUOUS"
    );
    assert_eq!(probe.api_calls.get(), 0);
    registry.instances.clear();
    probe.paths = vec![first, second];
    let result: Value = serde_json::to_value(
        doctor::execute(&store, &config, &registry, None, None, &probe).unwrap(),
    )
    .unwrap();
    assert_contract(&result);
    assert_eq!(result["data"]["canStart"], false);
    let qq = result["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["name"] == "qq")
        .unwrap();
    assert_eq!(qq["state"], "unknown");
    assert_eq!(qq["candidates"].as_array().unwrap().len(), 2);
}

fn assert_contract(value: &Value) {
    let schema = qq_cli::cli::registry::get("doctor").unwrap();
    let validator = jsonschema::validator_for(&schema["responseSchema"]).unwrap();
    assert!(
        validator.is_valid(value),
        "{:?}",
        validator
            .iter_errors(value)
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    );
    let mut bad = value.clone();
    bad["data"]["checks"][0]["extra"] = serde_json::json!(true);
    assert!(!validator.is_valid(&bad));
}

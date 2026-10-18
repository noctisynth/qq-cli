use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qq_cli::catalog::{self, Release};
use qq_cli::client::SessionInfo;
use qq_cli::compatibility::{EvidenceKind, Key, Verdict, Witness};
use qq_cli::config::Config;
use qq_cli::doctor::Probe;
use qq_cli::error::{AppError, Result};
use qq_cli::platform::Host;
use qq_cli::qq::{self, Application};
use qq_cli::startup;
use qq_cli::state::{Installation, Instance, Registry, Store};

#[path = "common/qq_fixture.rs"]
mod fixture;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn host() -> Host {
    Host {
        os: "windows".into(),
        os_build: Some(10000),
        native_arch: "x86_64".into(),
        process_arch: "x86_64".into(),
    }
}
struct Fake {
    paths: Vec<PathBuf>,
    artifact_ok: bool,
    artifact_calls: Cell<usize>,
    discovery_calls: Cell<usize>,
    metadata_calls: Cell<usize>,
}
impl Fake {
    fn new(path: PathBuf) -> Self {
        Self {
            paths: vec![path],
            artifact_ok: true,
            artifact_calls: Cell::new(0),
            discovery_calls: Cell::new(0),
            metadata_calls: Cell::new(0),
        }
    }
}
impl Probe for Fake {
    fn host(&self) -> Host {
        host()
    }
    fn qq_paths(&self) -> Result<Vec<PathBuf>> {
        self.discovery_calls.set(self.discovery_calls.get() + 1);
        Ok(self.paths.clone())
    }
    fn inspect_qq(&self, path: &Path, deadline: Instant) -> Result<Application> {
        self.metadata_calls.set(self.metadata_calls.get() + 1);
        qq::inspect(path, deadline)
    }
    fn artifact(&self, _: &Store, _: &Installation, _: Instant) -> Result<()> {
        self.artifact_calls.set(self.artifact_calls.get() + 1);
        if self.artifact_ok {
            Ok(())
        } else {
            Err(AppError::new(
                "INSTALLATION_MODIFIED",
                "Original fixture refusal",
                4,
            ))
        }
    }
    fn api(&self, _: &Store, _: &Config, _: &Instance, _: &str, _: Instant) -> Result<SessionInfo> {
        panic!("startup inspection must never contact an account API")
    }
}
fn release() -> Release {
    let mut release = catalog::lookup("v4.18.38").unwrap();
    // Only this synthetic probe test supplies an original-fixture basis. The
    // production catalog and public CLI retain the unresolved authorization gate.
    release.authorization_evidence = Some("original synthetic startup fixture");
    release
}
fn registry(root: &Path, release: &Release) -> Registry {
    Registry {
        schema_version: 1,
        instances: vec![],
        installations: vec![Installation {
            id: "original-installation".into(),
            version: release.version.into(),
            digest: release.archive.sha256.into(),
            directory: root.join("original-installation"),
        }],
    }
}
fn code(result: Result<startup::Inputs>) -> &'static str {
    result.err().expect("inspection must fail").code
}

#[test]
fn unconfirmed_authorization_and_expiry_precede_all_artifact_or_qq_access() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let probe = Fake::new(temp.path().join("not-opened/QQ.exe"));
    let release = catalog::lookup("v4.18.38").unwrap();
    assert_eq!(
        code(startup::inspect(
            &store,
            &Registry::default(),
            &release,
            &[],
            &probe,
            deadline()
        )),
        "LICENSE_UNCONFIRMED"
    );
    assert_eq!(
        code(startup::inspect(
            &store,
            &Registry::default(),
            &release,
            &[],
            &probe,
            Instant::now()
        )),
        "DEADLINE"
    );
    assert_eq!(probe.artifact_calls.get(), 0);
    assert_eq!(probe.discovery_calls.get(), 0);
    assert!(!store.root().exists());
}

#[test]
fn artifact_identity_ambiguity_and_integrity_stop_before_qq_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let release = release();
    let mut registered = registry(temp.path(), &release);
    let mut probe = Fake::new(temp.path().join("not-opened/QQ.exe"));
    assert_eq!(
        code(startup::inspect(
            &store,
            &Registry::default(),
            &release,
            &[],
            &probe,
            deadline()
        )),
        "INSTALLATION_NOT_FOUND"
    );
    registered.installations[0].digest = "0".repeat(64);
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "INSTALLATION_MODIFIED"
    );
    registered = registry(temp.path(), &release);
    registered
        .installations
        .push(registered.installations[0].clone());
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "INSTALLATION_AMBIGUOUS"
    );
    registered.installations.pop();
    probe.artifact_ok = false;
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "INSTALLATION_MODIFIED"
    );
    assert_eq!(probe.discovery_calls.get(), 0);
    assert!(!store.root().exists());
}

#[test]
fn discovery_is_bounded_and_never_selects_among_ambiguous_or_invalid_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let first = fixture::fixture(&temp.path().join("first"), false);
    let second = fixture::fixture(&temp.path().join("second"), true);
    let store = Store::new(temp.path().join("state"));
    let release = release();
    let registered = registry(temp.path(), &release);
    let mut probe = Fake::new(first.clone());
    probe.paths = vec![];
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "QQ_NOT_FOUND"
    );
    probe.paths = vec![first.clone(); 17];
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "QQ_DISCOVERY_LIMIT"
    );
    assert_eq!(probe.metadata_calls.get(), 0);
    probe.paths = vec![PathBuf::from("relative/QQ.exe")];
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "QQ_METADATA_INVALID"
    );
    probe.paths = vec![first, second];
    let error = startup::inspect(&store, &registered, &release, &[], &probe, deadline())
        .err()
        .unwrap();
    assert_eq!(error.code, "QQ_INSTALLATION_AMBIGUOUS");
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains(&temp.path().to_string_lossy().to_string())
    );
    assert!(!store.root().exists());
}

#[test]
fn only_exact_framework_evidence_passes_and_software_changes_invalidate_it() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(&temp.path().join("qq"), false);
    let application = qq::inspect(&executable, deadline()).unwrap();
    let release = release();
    let key = Key::from_installation(host(), &release, &application).unwrap();
    let mut witness = Witness {
        key,
        verdict: Verdict::Supported,
        reference: "original framework startup fixture".into(),
        kind: EvidenceKind::ShellSmoke,
    };
    let store = Store::new(temp.path().join("state"));
    let registered = registry(temp.path(), &release);
    let mut probe = Fake::new(executable.clone());
    probe.paths.push(executable); // Duplicate registry views do not create ambiguity.
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[],
            &probe,
            deadline()
        )),
        "COMPATIBILITY_UNKNOWN"
    );
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            std::slice::from_ref(&witness),
            &probe,
            deadline()
        )),
        "COMPATIBILITY_UNKNOWN"
    );
    witness.kind = EvidenceKind::FrameworkStartup;
    let inputs = startup::inspect(
        &store,
        &registered,
        &release,
        std::slice::from_ref(&witness),
        &probe,
        deadline(),
    )
    .unwrap();
    assert_eq!(inputs.application().fingerprint, application.fingerprint);
    assert_eq!(inputs.installation().id, "original-installation");
    assert_eq!(inputs.compatibility(), &witness.key);
    witness.verdict = Verdict::Unsupported;
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            std::slice::from_ref(&witness),
            &probe,
            deadline()
        )),
        "COMPATIBILITY_UNSUPPORTED"
    );
    witness.verdict = Verdict::Supported;
    witness.key.qq_fingerprint = "0".repeat(64);
    assert_eq!(
        code(startup::inspect(
            &store,
            &registered,
            &release,
            &[witness],
            &probe,
            deadline()
        )),
        "COMPATIBILITY_UNKNOWN"
    );
    assert!(!store.root().exists());
}

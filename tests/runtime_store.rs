#![cfg(windows)]
use qq_cli::{
    core::error::{AppError, Result},
    napcat::runtime::runtime_config::Ports,
    napcat::runtime::runtime_store::{self, Plan},
    storage::credentials::{Credentials, SystemCredentials},
    storage::references::References,
    storage::state::{Installation, Instance, Store},
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[path = "common/qq_fixture.rs"]
mod qq_fixture;

fn until() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn wait_original_exit(process: &qq_cli::native::process::Pinned, deadline: Instant) {
    loop {
        match process.verify(deadline) {
            Err(error) if error.code == "PROCESS_EXITED" => return,
            Ok(()) => (),
            Err(error) if error.code == "PROCESS_UNVERIFIABLE" => (),
            Err(error) => panic!("unexpected original process observation: {}", error.code),
        }
        assert!(
            Instant::now() < deadline,
            "original process did not signal exit"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[derive(Default)]
struct Memory {
    values: RefCell<BTreeMap<String, String>>,
    gets: Cell<usize>,
    sets: Cell<usize>,
    fail: Cell<Option<usize>>,
    after: bool,
}
impl Credentials for Memory {
    fn get(&self, key: &str) -> Result<String> {
        self.gets.set(self.gets.get() + 1);
        self.values.borrow().get(key).cloned().ok_or_else(|| {
            AppError::new("CREDENTIAL_NOT_FOUND", "Synthetic credential missing.", 4)
        })
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        let count = self.sets.get() + 1;
        self.sets.set(count);
        let fail = self.fail.get() == Some(count);
        if !fail || self.after {
            self.values.borrow_mut().insert(key.into(), value.into());
        }
        if fail {
            self.fail.set(None);
            Err(AppError::new(
                "SYNTHETIC_VAULT_FAILURE",
                "Synthetic credential write failed.",
                6,
            ))
        } else {
            Ok(())
        }
    }
    fn delete(&self, key: &str) -> Result<()> {
        self.values.borrow_mut().remove(key);
        Ok(())
    }
}
fn fixture(root: &Path) -> (Store, References, Plan) {
    let store = Store::new(root.join("state"));
    let plan = Plan {
        instance_id: "instance-example".into(),
        run_id: "run-example".into(),
        version: "v4.18.38".into(),
        ports: Ports {
            http: 19001,
            events: 19002,
            control: 19003,
        },
    };
    let mut transaction = store.transaction(until()).unwrap();
    transaction.registry.installations.push(Installation {
        id: "install-example".into(),
        version: plan.version.clone(),
        digest: "a".repeat(64),
        directory: root.join("unused-installation"),
    });
    transaction.registry.instances.push(Instance {
        id: plan.instance_id.clone(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:19001/".into(),
        ws_endpoint: Some("ws://127.0.0.1:19002/event".into()),
        credential_key: runtime_store::api_credential_key(&plan.instance_id, &plan.run_id).unwrap(),
        account: None,
        run_id: plan.run_id.clone(),
        state: "preparing".into(),
        process_id: None,
        process_started: None,
    });
    transaction.commit().unwrap();
    let refs = References::with_key(store.root().join("references"), [95; 32]);
    (store, refs, plan)
}
fn directory(store: &Store, plan: &Plan) -> PathBuf {
    store.root().canonicalize().unwrap().join("runtimes").join(
        runtime_store::api_credential_key(&plan.instance_id, &plan.run_id)
            .unwrap()
            .strip_prefix("runtime-api-")
            .unwrap(),
    )
}
fn bundle_key(plan: &Plan) -> String {
    runtime_store::api_credential_key(&plan.instance_id, &plan.run_id)
        .unwrap()
        .replacen("runtime-api-", "runtime-bundle-", 1)
}

#[test]
fn locked_prepared_verification_never_repairs_or_regenerates_launch_material() {
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, plan) = fixture(temp.path());
    let vault = Memory::default();
    let prepared = runtime_store::prepare(&store, &refs, &vault, &plan, until()).unwrap();
    let sets = vault.sets.get();
    let registry = std::fs::read(store.root().join("registry.json")).unwrap();
    let mut tx = store.transaction(until()).unwrap();
    for phase in ["prepared", "starting"] {
        tx.registry.instances[0].state = phase.into();
        let checked = runtime_store::verify_prepared(&tx, &refs, &vault, &plan, until()).unwrap();
        assert_eq!(checked.directory(), prepared.directory());
        assert_eq!(checked.bundle().api_token(), prepared.bundle().api_token());
        assert_eq!(vault.sets.get(), sets);
    }
    let gets = vault.gets.get();
    tx.registry.instances[0].state = "ready".into();
    assert!(runtime_store::verify_prepared(&tx, &refs, &vault, &plan, until()).is_err());
    assert_eq!(vault.gets.get(), gets);
    tx.registry.instances[0].state = "starting".into();
    let file = prepared
        .directory()
        .join("config")
        .join(prepared.bundle().files().next().unwrap().name());
    let original = std::fs::read(&file).unwrap();
    std::fs::write(&file, b"original changed config").unwrap();
    assert!(runtime_store::verify_prepared(&tx, &refs, &vault, &plan, until()).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), b"original changed config");
    std::fs::write(&file, original).unwrap();
    let key = prepared.api_credential_key();
    let token = vault.values.borrow_mut().remove(key).unwrap();
    assert!(runtime_store::verify_prepared(&tx, &refs, &vault, &plan, until()).is_err());
    assert!(!vault.values.borrow().contains_key(key));
    vault.values.borrow_mut().insert(key.into(), token);
    tx.registry.installations[0].digest = "b".repeat(64);
    assert!(runtime_store::verify_prepared(&tx, &refs, &vault, &plan, until()).is_err());
    assert_eq!(vault.sets.get(), sets);
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        registry
    );
    assert!(!store.root().join("start-requests").exists());
    assert!(!store.root().join("run-jobs").exists());
    drop(tx);
    let target = qq_cli::napcat::runtime::start_intent::Target {
        instance_id: plan.instance_id.clone(),
        run_id: plan.run_id.clone(),
        installation_id: "install-example".into(),
        installation_digest: "a".repeat(64),
        input_hash: "c".repeat(64),
    };
    let qq_cli::napcat::runtime::start_intent::Begin::Fresh(intent) =
        qq_cli::napcat::runtime::start_intent::begin(
            &store,
            &refs,
            "original-material-dispatch",
            &target,
            until(),
        )
        .unwrap()
    else {
        panic!("fresh original dispatch")
    };
    let calls = Cell::new(0);
    let receipt = intent
        .dispatch(|locked| {
            let material = runtime_store::verify_prepared(locked, &refs, &vault, &plan, until())?;
            assert_eq!(material.directory(), prepared.directory());
            calls.set(calls.get() + 1);
            Ok(()) // Original callback only; no process is created.
        })
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        receipt.outcome,
        qq_cli::napcat::runtime::start_intent::Outcome::Dispatched
    );
    assert_eq!(vault.sets.get(), sets);
    assert!(!store.root().join("run-jobs").exists());
}

#[test]
fn live_control_material_requires_original_claim_marker_and_secrets_without_repairs() {
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, plan) = fixture(temp.path());
    let vault = Memory::default();
    let prepared = runtime_store::prepare(&store, &refs, &vault, &plan, until()).unwrap();
    std::fs::write(
        prepared.directory().join("logs/original-live-log"),
        b"private ignored log",
    )
    .unwrap();
    std::fs::write(
        prepared.directory().join("cache/original-live-cache"),
        b"private ignored bytes",
    )
    .unwrap();
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.instances[0].state = "login_required".into();
    let instance = tx.registry.instances[0].clone();
    let installation = tx.registry.installations[0].clone();
    let sets = vault.sets.get();
    let before = std::fs::read(store.root().join("registry.json")).unwrap();
    let material =
        runtime_store::recover_control(&tx, &refs, &vault, &instance, &installation, until())
            .unwrap();
    assert_eq!(material.endpoint(), "http://127.0.0.1:19003/");
    assert_eq!(material.token(), prepared.bundle().control_token());
    assert_ne!(material.token(), prepared.bundle().api_token());
    let gets = vault.gets.get();
    let mut changed = instance.clone();
    changed.run_id = "other-run".into();
    assert!(
        runtime_store::recover_control(&tx, &refs, &vault, &changed, &installation, until())
            .is_err()
    );
    assert_eq!(gets, vault.gets.get());
    let marker = prepared.directory().join(".qq-cli-runtime.json");
    let original = std::fs::read(&marker).unwrap();
    std::fs::write(&marker, b"{}").unwrap();
    assert!(
        runtime_store::recover_control(&tx, &refs, &vault, &instance, &installation, until())
            .is_err()
    );
    assert_eq!(gets, vault.gets.get());
    std::fs::write(&marker, original).unwrap();
    let raw = vault
        .values
        .borrow_mut()
        .remove(&bundle_key(&plan))
        .unwrap();
    assert!(
        runtime_store::recover_control(&tx, &refs, &vault, &instance, &installation, until())
            .is_err()
    );
    assert!(!vault.values.borrow().contains_key(&bundle_key(&plan)));
    vault.values.borrow_mut().insert(bundle_key(&plan), raw);
    let mut changed_installation = installation.clone();
    changed_installation.digest = "b".repeat(64);
    assert!(
        runtime_store::recover_control(
            &tx,
            &refs,
            &vault,
            &instance,
            &changed_installation,
            until()
        )
        .is_err()
    );
    assert_eq!(vault.sets.get(), sets);
    assert_eq!(
        std::fs::read(store.root().join("registry.json")).unwrap(),
        before
    );
    assert!(
        runtime_store::recover_control(&tx, &refs, &vault, &instance, &installation, until())
            .is_ok()
    );
}

#[test]
fn complete_dispatch_inputs_bind_runtime_and_filtered_environment_before_native_creation() {
    original_dispatch_fixture(Scenario::Dispatch);
}

#[test]
fn explicit_start_prepares_dispatches_retains_and_reuses_one_account_free_instance() {
    original_dispatch_fixture(Scenario::Explicit);
}

#[test]
fn automatic_start_creates_one_account_free_instance_and_reuses_its_checkpoint() {
    original_dispatch_fixture(Scenario::Automatic);
}
#[test]
fn automatic_start_resumes_its_registered_preparation_with_the_original_key() {
    original_dispatch_fixture(Scenario::Resume);
}
#[test]
fn selected_stopped_start_keeps_instance_and_launches_one_successor_run() {
    original_dispatch_fixture(Scenario::Stopped);
}
#[test]
fn automatic_stopped_start_keeps_instance_and_launches_one_successor_run() {
    original_dispatch_fixture(Scenario::StoppedAutomatic);
}
#[test]
fn automatic_start_recovers_a_successor_preparation_instead_of_the_old_default() {
    original_dispatch_fixture(Scenario::SuccessorResume);
}
#[derive(Clone, Copy)]
enum Scenario {
    Dispatch,
    Explicit,
    Automatic,
    Resume,
    Stopped,
    StoppedAutomatic,
    SuccessorResume,
}
fn original_dispatch_fixture(scenario: Scenario) {
    use qq_cli::{
        napcat::compatibility::{EvidenceKind, Key, Verdict, Witness},
        napcat::doctor::Probe,
        napcat::install::catalog,
        napcat::runtime::dispatch,
        napcat::runtime::startup,
    };
    use std::ffi::OsString;
    struct Original(PathBuf);
    impl Probe for Original {
        fn host(&self) -> qq_cli::native::platform::Host {
            qq_cli::native::platform::Host {
                os: "windows".into(),
                os_build: Some(10000),
                native_arch: "x86_64".into(),
                process_arch: "x86_64".into(),
            }
        }
        fn qq_paths(&self) -> Result<Vec<PathBuf>> {
            Ok(vec![self.0.clone()])
        }
        fn inspect_qq(
            &self,
            path: &Path,
            deadline: Instant,
        ) -> Result<qq_cli::napcat::qq::Application> {
            qq_cli::napcat::qq::inspect(path, deadline)
        }
        fn artifact(&self, _: &Store, _: &Installation, _: Instant) -> Result<()> {
            Ok(())
        }
        fn api(
            &self,
            _: &Store,
            _: &qq_cli::storage::config::Config,
            _: &Instance,
            _: &str,
            _: Instant,
        ) -> Result<qq_cli::transport::client::SessionInfo> {
            panic!("no account API during dispatch checks")
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, mut plan) = fixture(temp.path());
    let reservations = (0..3)
        .map(|_| std::net::TcpListener::bind("127.0.0.1:0").unwrap())
        .collect::<Vec<_>>();
    plan.ports = Ports {
        http: reservations[0].local_addr().unwrap().port(),
        events: reservations[1].local_addr().unwrap().port(),
        control: reservations[2].local_addr().unwrap().port(),
    };
    let executable = qq_fixture::fixture(&temp.path().join("original-qq"), false);
    let probe = Original(executable.clone());
    let mut release = catalog::lookup(&plan.version).unwrap();
    release.authorization_evidence = Some("original dispatch fixture, not upstream authorization");
    let installation = temp.path().join("original-installation");
    std::fs::create_dir(&installation).unwrap();
    for name in ["napimain.exe", "napiloader.dll", "nativeLoader.cjs"] {
        std::fs::write(installation.join(name), b"original fixture, never executed").unwrap();
    }
    // Original stand-in: checks the three-argument ABI, never loads QQ/DLL/JS.
    let source = temp.path().join("original_child.rs");
    std::fs::write(
        &source,
        r#"
fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("NAPCAT_WORKDIR").unwrap());
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new("--original-service")) {
        use std::os::windows::process::CommandExt;
        assert_eq!(std::env::args_os().count(), 4);
        let _child = std::process::Command::new(std::env::args_os().nth(1).unwrap())
            .arg("--original-service").creation_flags(0x08000000).spawn().unwrap();
        let end = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !root.join("tmp/original-loader-exit").exists() && std::time::Instant::now() < end {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        return;
    }
    use std::io::Read;
    fn port(root: &std::path::Path, file: &str) -> u16 {
        let text = std::fs::read_to_string(root.join("config").join(file)).unwrap();
        let after = text.split_once("\"port\":").unwrap().1.trim_start();
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().unwrap()
    }
    let control = std::net::TcpListener::bind(("127.0.0.1", port(&root, "webui.json"))).unwrap();
    control.set_nonblocking(true).unwrap();
    let mut api = None;
    std::fs::write(root.join("logs/original-started"), b"original child").unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut connections = 0;
    while !root.join("tmp/original-release").exists() && std::time::Instant::now() < end {
        if api.is_none() && root.join("tmp/original-enable-api").exists() {
            let listener = std::net::TcpListener::bind(("127.0.0.1", port(&root, "onebot11.json"))).unwrap();
            listener.set_nonblocking(true).unwrap();
            api = Some(listener);
            std::fs::write(root.join("logs/original-api-ready"), b"original API").unwrap();
        }
        for listener in std::iter::once(&control).chain(api.iter()) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
                assert_eq!(stream.read(&mut [0; 1]).unwrap(), 0);
                connections += 1;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => (),
            Err(error) => panic!("original listener: {error}"),
        }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    std::fs::write(root.join("logs/original-connections"), connections.to_string()).unwrap();
}
"#,
    )
    .unwrap();
    let compiled = std::process::Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "original_dispatch_child"])
        .arg(&source)
        .arg("-o")
        .arg(installation.join("napimain.exe"))
        .output()
        .unwrap();
    assert!(compiled.status.success(), "original helper did not compile");
    std::fs::copy(installation.join("napimain.exe"), &executable).unwrap();
    let mut tx = store.transaction(until()).unwrap();
    tx.registry.installations[0].directory = installation;
    tx.registry.installations[0].digest = release.archive.sha256.into();
    tx.registry.instances[0].endpoint = format!("http://127.0.0.1:{}/", plan.ports.http);
    tx.registry.instances[0].ws_endpoint =
        Some(format!("ws://127.0.0.1:{}/event", plan.ports.events));
    tx.commit().unwrap();
    let witness = Witness {
        key: Key::from_installation(
            probe.host(),
            &release,
            &qq_cli::napcat::qq::inspect(&executable, until()).unwrap(),
        )
        .unwrap(),
        verdict: Verdict::Supported,
        reference: "original dispatch witness".into(),
        kind: EvidenceKind::FrameworkStartup,
    };
    let witnesses = [witness];
    if !matches!(scenario, Scenario::Dispatch) {
        drop(reservations);
        let automatic = matches!(
            scenario,
            Scenario::Automatic
                | Scenario::Resume
                | Scenario::StoppedAutomatic
                | Scenario::SuccessorResume
        );
        if matches!(scenario, Scenario::Automatic | Scenario::Resume) {
            let mut tx = store.transaction(until()).unwrap();
            tx.registry.instances.clear();
            tx.commit().unwrap();
        }
        let vault = Memory::default();
        let environment = BTreeMap::from([(
            OsString::from("SYSTEMROOT"),
            std::env::var_os("SystemRoot").unwrap(),
        )]);
        let context = qq_cli::napcat::runtime::start_engine::Context {
            store: &store,
            references: &refs,
            credentials: &vault,
            release: &release,
            witnesses: &witnesses,
            probe: &probe,
            environment: &environment,
        };
        use base64::Engine;
        vault.values.borrow_mut().insert(
            "reference-signing-key".into(),
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([95; 32]),
        );
        let mut original_key = None;
        let mut existing_source = None;
        if matches!(
            scenario,
            Scenario::Stopped | Scenario::StoppedAutomatic | Scenario::SuccessorResume
        ) {
            let current = store.read().unwrap().instances.remove(0);
            qq_cli::napcat::runtime::stop::execute(&store, &refs, &current, until()).unwrap();
            let stopped = store.read().unwrap().instances.remove(0);
            existing_source = Some(stopped.clone());
            if matches!(scenario, Scenario::SuccessorResume) {
                let inputs = startup::inspect(
                    &store,
                    &store.read().unwrap(),
                    &release,
                    &witnesses,
                    &probe,
                    until(),
                )
                .unwrap();
                let next = inputs
                    .reserve_next_run(
                        &store,
                        &refs,
                        &stopped,
                        &qq_cli::napcat::runtime::instance_creation::Evidence {
                            release: &release,
                            witnesses: &witnesses,
                            probe: &probe,
                        },
                        until(),
                    )
                    .unwrap();
                original_key = Some(next.request_id);
                drop(next.creation);
            }
        }
        if matches!(scenario, Scenario::Resume) {
            let qq_cli::napcat::runtime::default_start::Selection::Create { request_id } =
                qq_cli::napcat::runtime::default_start::select(
                    &store,
                    &refs,
                    release.version,
                    until(),
                )
                .unwrap()
            else {
                panic!("original default creation")
            };
            let inputs = startup::inspect(
                &store,
                &store.read().unwrap(),
                &release,
                &witnesses,
                &probe,
                until(),
            )
            .unwrap();
            let reserved = inputs
                .reserve_instance(
                    &store,
                    &refs,
                    &request_id,
                    &qq_cli::napcat::runtime::instance_creation::Evidence {
                        release: &release,
                        witnesses: &witnesses,
                        probe: &probe,
                    },
                    until(),
                )
                .unwrap();
            drop(reserved); // Simulated CLI exit after registration, before preparation.
            let pending = serde_json::to_value(
                qq_cli::requests::query::get(&store, &request_id, None, &vault, until()).unwrap(),
            )
            .unwrap();
            assert_eq!(pending["data"]["outcome"], "pending");
            assert_eq!(pending["data"]["identityVerified"], false);
            assert!(!store.root().join("start-requests").exists());
            original_key = Some(request_id);
        }
        let start = || {
            if matches!(scenario, Scenario::Stopped) {
                qq_cli::napcat::runtime::start_engine::selected(
                    &context,
                    existing_source.as_ref().unwrap(),
                    until(),
                )
            } else if automatic {
                qq_cli::napcat::runtime::start_engine::automatic(&context, until())
            } else {
                qq_cli::napcat::runtime::start_engine::explicit(
                    &context,
                    "original-engine",
                    until(),
                )
            }
        };
        let started = start().unwrap();
        assert_eq!(
            started.reused,
            matches!(scenario, Scenario::Resume | Scenario::SuccessorResume)
        );
        if let Some(source) = &existing_source {
            assert_eq!(started.instance.id, source.id);
            assert_ne!(started.instance.run_id, source.run_id);
        }
        if let Some(original_key) = original_key {
            assert_eq!(started.request_id.as_deref(), Some(original_key.as_str()));
        }
        assert_eq!(started.instance.state, "login_required");
        assert!(started.instance.account.is_none());
        let request_id = started.request_id.as_deref().unwrap();
        let queried = serde_json::to_value(
            qq_cli::requests::query::get(&store, request_id, None, &vault, until()).unwrap(),
        )
        .unwrap();
        let schema = qq_cli::cli::registry::get("request.get").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&queried),
            "{queried}"
        );
        assert_eq!(queried["data"]["outcome"], "dispatched");
        assert_eq!(queried["data"]["instanceId"], started.instance.id);
        assert_eq!(queried["data"]["identityVerified"], false);
        assert!(queried.get("meta").is_none());
        assert_eq!(
            qq_cli::requests::ledger::begin(
                &store,
                request_id,
                "send",
                &qq_cli::core::domain::Scope {
                    instance_id: started.instance.id.clone(),
                    account: "10001".into()
                },
                &serde_json::json!({}),
                until()
            )
            .err()
            .unwrap()
            .code,
            "REQUEST_CONFLICT"
        );
        if matches!(scenario, Scenario::Explicit) {
            let pointer = std::fs::read_dir(store.root().join("start-requests"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let original = std::fs::read(&pointer).unwrap();
            let mut record =
                qq_cli::napcat::runtime::start_intent::get(&store, &refs, request_id).unwrap();
            record.target.run_id = "other-original-run".into();
            qq_cli::storage::state::atomic_json(&pointer, &serde_json::json!({"schemaVersion":1,"reference":refs.issue("s1", &record).unwrap()})).unwrap();
            assert_eq!(
                qq_cli::requests::query::get(&store, request_id, None, &vault, until())
                    .err()
                    .unwrap()
                    .code,
                "REQUEST_RECORD_CONFLICT"
            );
            std::fs::write(pointer, original).unwrap();
        }
        let challenge = qq_cli::app::runtime::response(
            &store,
            &vault,
            &qq_cli::storage::config::Config::default(),
            qq_cli::napcat::runtime::start_engine::Started {
                instance: started.instance.clone(),
                reused: started.reused,
                request_id: started.request_id.clone(),
            },
            until(),
        )
        .err()
        .unwrap();
        assert_eq!(challenge.code, "AUTH_REQUIRED");
        assert_eq!(
            challenge.details.unwrap()["account"],
            serde_json::Value::Null
        );
        assert_eq!(
            challenge.next_action.unwrap()["args"]["instance"],
            started.instance.id
        );
        let mut created_plan = plan.clone();
        created_plan.instance_id = started.instance.id.clone();
        created_plan.run_id = started.instance.run_id.clone();
        let runtime = directory(&store, &created_plan);
        assert!(runtime.join("logs/original-started").exists());
        assert!(!runtime.join("logs/original-api-ready").exists());
        let secrets = vault.values.borrow().clone();
        let repeated = start().unwrap();
        assert!(repeated.reused);
        assert!(repeated.instance == started.instance);
        assert!(*vault.values.borrow() == secrets);
        assert_eq!(
            store.read().unwrap().instances.len(),
            if automatic || existing_source.is_some() {
                1
            } else {
                2
            }
        );
        assert_eq!(
            std::fs::read_dir(store.root().join("run-jobs"))
                .unwrap()
                .count(),
            1
        );
        let mut tx = store.transaction(until()).unwrap();
        tx.registry
            .instances
            .iter_mut()
            .find(|i| i.id == started.instance.id)
            .unwrap()
            .state = "starting".into();
        tx.commit().unwrap();
        let error = start().err().unwrap();
        assert_eq!(error.code, "START_OUTCOME_UNKNOWN");
        assert!(!error.retry_safe);
        assert_eq!(
            error.next_action.unwrap()["args"]["instance"],
            started.instance.id
        );
        assert_eq!(
            std::fs::read_dir(store.root().join("run-jobs"))
                .unwrap()
                .count(),
            1
        );
        // Release only this original fixture. No installed QQ or real session is involved.
        std::fs::write(
            runtime.join("tmp/original-loader-exit"),
            b"original release",
        )
        .unwrap();
        std::fs::write(runtime.join("tmp/original-release"), b"original release").unwrap();
        let end = until();
        while !runtime.join("logs/original-connections").exists() {
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            std::fs::read_to_string(runtime.join("logs/original-connections")).unwrap(),
            "1"
        );
        return;
    }
    let inputs = startup::inspect(
        &store,
        &store.read().unwrap(),
        &release,
        &witnesses,
        &probe,
        until(),
    )
    .unwrap();
    let vault = Memory::default();
    runtime_store::prepare(&store, &refs, &vault, &plan, until()).unwrap();
    let inherited = BTreeMap::from([(
        OsString::from("SYSTEMROOT"),
        std::env::var_os("SystemRoot").unwrap(),
    )]);
    let request = |env| dispatch::Request {
        inputs: &inputs,
        release: &release,
        witnesses: &witnesses,
        probe: &probe,
        references: &refs,
        credentials: &vault,
        runtime: &plan,
        inherited_environment: env,
    };
    let tx = store.transaction(until()).unwrap();
    let checked = dispatch::check_in(&tx, &request(&inherited), until()).unwrap();
    assert_eq!(checked.spec().arguments.len(), 3);
    assert!(checked.spec().program.ends_with("napimain.exe"));
    assert!(
        checked
            .spec()
            .environment
            .contains_key(&OsString::from("NAPCAT_WORKDIR"))
    );
    let target = checked.target().clone();
    let mut ignored = inherited.clone();
    ignored.insert("NODE_OPTIONS".into(), "original ignored override".into());
    assert_eq!(
        dispatch::check_in(&tx, &request(&ignored), until())
            .unwrap()
            .target(),
        &target
    );
    let mut changed = inherited.clone();
    changed.insert(
        "APPDATA".into(),
        "original changed inherited location".into(),
    );
    assert_ne!(
        dispatch::check_in(&tx, &request(&changed), until())
            .unwrap()
            .target()
            .input_hash,
        target.input_hash
    );
    drop(tx);
    let qq_cli::napcat::runtime::start_intent::Begin::Fresh(intent) =
        qq_cli::napcat::runtime::start_intent::begin(
            &store,
            &refs,
            "original-dispatch-check",
            &target,
            until(),
        )
        .unwrap()
    else {
        panic!("fresh original intent")
    };
    let mut wrong = target.clone();
    wrong.input_hash = "0".repeat(64);
    let mut running = None;
    let receipt = intent
        .dispatch(|tx| {
            let error = dispatch::launch_in(
                tx,
                &request(&inherited),
                "original-dispatch-check",
                &wrong,
                until(),
            )
            .err()
            .unwrap();
            assert_eq!(error.code, "START_INPUTS_CHANGED");
            assert!(!store.root().join("run-jobs").exists());
            let busy = dispatch::launch_in(
                tx,
                &request(&inherited),
                "original-dispatch-check",
                &target,
                until(),
            )
            .err()
            .unwrap();
            assert_eq!(busy.code, "ENDPOINT_IN_USE");
            assert!(!store.root().join("run-jobs").exists());
            drop(reservations);
            running = Some(dispatch::launch_in(
                tx,
                &request(&inherited),
                "original-dispatch-check",
                &target,
                until(),
            )?);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        receipt.outcome,
        qq_cli::napcat::runtime::start_intent::Outcome::Dispatched
    );
    let child = running.unwrap();
    let runtime = directory(&store, &plan);
    let end = until();
    while !runtime.join("logs/original-started").exists() {
        assert!(Instant::now() < end, "original child did not start");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(child.job().contains(child.root(), until()).unwrap());
    let mut tx = store.transaction(until()).unwrap();
    let endpoint = tx.registry.instances[0].endpoint.clone();
    tx.registry.instances[0].endpoint = "http://127.0.0.1:1/".into();
    assert_eq!(
        qq_cli::napcat::runtime::run_job::discover_service(
            &tx,
            &refs,
            &target,
            child.job(),
            until()
        )
        .err()
        .unwrap()
        .code,
        "RUN_JOB_CONFLICT"
    );
    tx.registry.instances[0].endpoint = endpoint;
    let wrong_job = qq_cli::native::job::Job::create(until()).unwrap();
    assert_eq!(
        qq_cli::napcat::runtime::run_job::discover_service(
            &tx,
            &refs,
            &target,
            &wrong_job,
            until()
        )
        .err()
        .unwrap()
        .code,
        "RUN_JOB_CONFLICT"
    );
    let retained = qq_cli::napcat::runtime::run_job::retain_prelogin(
        &tx,
        &refs,
        &target,
        child.job(),
        until(),
    )
    .unwrap();
    let service_pid = retained.owner().identity().process_id();
    let control = qq_cli::napcat::runtime::run_job::reopen_control(
        &tx,
        &refs,
        &tx.registry.instances[0],
        &tx.registry.installations[0],
        until(),
    )
    .unwrap();
    assert_eq!(
        control.service().owner().identity().process_id(),
        service_pid
    );
    assert_eq!(
        control.endpoint(),
        format!("http://127.0.0.1:{}/", plan.ports.control)
    );
    drop(control);
    let retained_again = qq_cli::napcat::runtime::run_job::retain_prelogin(
        &tx,
        &refs,
        &target,
        child.job(),
        until(),
    )
    .unwrap();
    assert!(retained.owner().identity() == retained_again.owner().identity());
    let retention_pointer = std::fs::read_dir(store.root().join("retentions"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let saved_retention = std::fs::read(&retention_pointer).unwrap();
    std::fs::remove_file(&retention_pointer).unwrap();
    assert!(
        qq_cli::napcat::runtime::run_job::reopen_control(
            &tx,
            &refs,
            &tx.registry.instances[0],
            &tx.registry.installations[0],
            until()
        )
        .is_err()
    );
    assert!(
        qq_cli::napcat::runtime::run_job::retain_prelogin(
            &tx,
            &refs,
            &target,
            child.job(),
            until()
        )
        .is_err()
    );
    assert!(!retention_pointer.exists());
    std::fs::write(&retention_pointer, saved_retention).unwrap();
    drop(retained_again);
    drop(retained);
    drop(tx);
    std::fs::write(
        runtime.join("tmp/original-loader-exit"),
        b"original loader release",
    )
    .unwrap();
    wait_original_exit(child.root(), end);
    drop(child); // All parent handles for this Job are now closed before API exists.
    std::fs::write(
        runtime.join("tmp/original-enable-api"),
        b"original login completion",
    )
    .unwrap();
    while !runtime.join("logs/original-api-ready").exists() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    use base64::Engine;
    vault.values.borrow_mut().insert(
        "reference-signing-key".into(),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([95; 32]),
    );
    let registry = store.read().unwrap();
    let _context = qq_cli::app::backend::Context::discover_pending(
        &store,
        &registry,
        &vault,
        &registry.instances[0],
        until(),
    )
    .unwrap();
    let tx = store.transaction(until()).unwrap();
    let service = qq_cli::napcat::runtime::run_job::discover_registered(
        &tx,
        &refs,
        &tx.registry.instances[0],
        &tx.registry.installations[0],
        until(),
    )
    .unwrap();
    assert_eq!(service.owner().identity().process_id(), service_pid);
    let recovered = qq_cli::napcat::runtime::run_job::discover_registered(
        &tx,
        &refs,
        &tx.registry.instances[0],
        &tx.registry.installations[0],
        until(),
    )
    .unwrap();
    assert!(recovered.owner().identity() == service.owner().identity());
    drop(tx);
    std::fs::write(runtime.join("tmp/original-release"), b"original release").unwrap();
    wait_original_exit(service.owner(), end);
    assert_eq!(
        std::fs::read_to_string(runtime.join("logs/original-connections")).unwrap(),
        "2"
    );
}

#[test]
fn prepared_runtime_uses_isolated_native_credentials_and_reuses_its_original_material() {
    struct Cleanup {
        vault: SystemCredentials,
        keys: Vec<String>,
    }
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for key in &self.keys {
                let _ = self.vault.delete(key);
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, plan) = fixture(temp.path());
    let key = runtime_store::api_credential_key(&plan.instance_id, &plan.run_id).unwrap();
    let vault = SystemCredentials::new(store.root()).unwrap();
    let keys = vec![key.clone(), bundle_key(&plan)];
    // Never claim cleanup ownership over a pre-existing credential entry.
    for key in &keys {
        assert_eq!(vault.get(key).err().unwrap().code, "CREDENTIAL_NOT_FOUND");
    }
    let cleanup = Cleanup { vault, keys };
    let prepared = runtime_store::prepare(&store, &refs, &cleanup.vault, &plan, until()).unwrap();
    assert!(
        prepared
            .directory()
            .starts_with(store.root().canonicalize().unwrap())
    );
    assert_eq!(store.read().unwrap().instances[0].state, "prepared");
    assert_eq!(
        cleanup.vault.get(&key).unwrap(),
        prepared.bundle().api_token()
    );
    let body = cleanup.vault.get(&bundle_key(&plan)).unwrap();
    let repeated = runtime_store::prepare(&store, &refs, &cleanup.vault, &plan, until()).unwrap();
    assert!(prepared.bundle().api_token() == repeated.bundle().api_token());
    assert_eq!(prepared.directory(), repeated.directory());
    assert!(body == cleanup.vault.get(&bundle_key(&plan)).unwrap());
    for file in prepared.bundle().files() {
        let mut expected = Vec::new();
        file.write_to(&mut expected).unwrap();
        assert_eq!(
            std::fs::read(prepared.directory().join("config").join(file.name())).unwrap(),
            expected
        );
    }
    let registry = std::fs::read_to_string(store.root().join("registry.json")).unwrap();
    assert!(!registry.contains(prepared.bundle().api_token()));
    assert!(!registry.contains(prepared.bundle().control_token()));
}

#[test]
fn uncertain_credential_writes_resume_without_replacing_committed_secrets() {
    for after in [false, true] {
        for failure in [1, 2] {
            let temp = tempfile::tempdir().unwrap();
            let (store, refs, plan) = fixture(temp.path());
            let memory = Memory {
                fail: Cell::new(Some(failure)),
                after,
                ..Memory::default()
            };
            assert_eq!(
                runtime_store::prepare(&store, &refs, &memory, &plan, until())
                    .err()
                    .unwrap()
                    .code,
                "SYNTHETIC_VAULT_FAILURE"
            );
            assert!(!directory(&store, &plan).exists());
            let committed = memory.values.borrow().clone();
            let prepared = runtime_store::prepare(&store, &refs, &memory, &plan, until()).unwrap();
            for (key, value) in committed {
                assert!(memory.values.borrow().get(&key) == Some(&value));
            }
            assert!(
                memory.get(prepared.api_credential_key()).unwrap() == prepared.bundle().api_token()
            );
        }
    }
}

#[test]
fn claimed_partial_files_and_publication_before_registry_commit_resume_safely() {
    let temp = tempfile::tempdir().unwrap();
    let (store, refs, plan) = fixture(temp.path());
    let memory = Memory::default();
    let prepared = runtime_store::prepare(&store, &refs, &memory, &plan, until()).unwrap();
    let original = memory.values.borrow().clone();
    let final_path = prepared.directory().to_owned();
    let stage = final_path.with_extension("stage");
    let root = store.root().canonicalize().unwrap();
    assert!(final_path.canonicalize().unwrap().starts_with(&root));
    assert!(
        stage
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .starts_with(&root)
    );
    std::fs::rename(&final_path, &stage).unwrap();
    std::fs::remove_file(stage.join(".qq-cli-runtime.json")).unwrap();
    let file = stage.join("config/onebot11.json");
    let bytes = std::fs::read(&file).unwrap();
    std::fs::write(&file, &bytes[..17]).unwrap();
    std::fs::remove_file(stage.join("config/webui.json")).unwrap();
    let mut transaction = store.transaction(until()).unwrap();
    transaction.registry.instances[0].state = "preparing".into();
    transaction.commit().unwrap();
    runtime_store::prepare(&store, &refs, &memory, &plan, until()).unwrap();
    assert_eq!(
        std::fs::read(final_path.join("config/onebot11.json")).unwrap(),
        bytes
    );
    assert!(!stage.exists());
    assert!(*memory.values.borrow() == original);
    let mut transaction = store.transaction(until()).unwrap();
    transaction.registry.instances[0].state = "preparing".into();
    transaction.commit().unwrap();
    runtime_store::prepare(&store, &refs, &memory, &plan, until()).unwrap();
    assert!(*memory.values.borrow() == original);
}

#[test]
fn unknown_directories_entries_and_changed_files_are_preserved() {
    for mode in [
        "orphan-directory",
        "orphan-key",
        "extra",
        "changed",
        "missing-secrets",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, plan) = fixture(temp.path());
        let memory = Memory::default();
        let path = directory(&store, &plan);
        match mode {
            "orphan-directory" => {
                std::fs::create_dir_all(&path).unwrap();
                std::fs::write(path.join("unknown"), b"preserve synthetic bytes").unwrap();
            }
            "orphan-key" => {
                memory
                    .values
                    .borrow_mut()
                    .insert(bundle_key(&plan), "unrelated synthetic credential".into());
            }
            _ => {
                runtime_store::prepare(&store, &refs, &memory, &plan, until()).unwrap();
                match mode {
                    "extra" => {
                        std::fs::write(path.join("config/extra.json"), b"preserve synthetic bytes")
                            .unwrap()
                    }
                    "changed" => std::fs::write(
                        path.join("config/onebot11.json"),
                        b"preserve synthetic bytes",
                    )
                    .unwrap(),
                    _ => {
                        memory.values.borrow_mut().clear();
                    }
                }
            }
        }
        let original = memory.values.borrow().clone();
        let error = runtime_store::prepare(&store, &refs, &memory, &plan, until())
            .err()
            .unwrap();
        assert!(matches!(
            error.code,
            "RUNTIME_PREPARATION_CONFLICT" | "RUNTIME_CREDENTIAL_CONFLICT"
        ));
        assert!(*memory.values.borrow() == original);
        if mode == "orphan-directory" {
            assert_eq!(memory.gets.get(), 0);
            assert_eq!(
                std::fs::read(path.join("unknown")).unwrap(),
                b"preserve synthetic bytes"
            );
        }
        if mode == "extra" {
            assert_eq!(
                std::fs::read(path.join("config/extra.json")).unwrap(),
                b"preserve synthetic bytes"
            );
        }
        if mode == "changed" {
            assert_eq!(
                std::fs::read(path.join("config/onebot11.json")).unwrap(),
                b"preserve synthetic bytes"
            );
        }
    }
}

#[test]
fn wrong_run_phase_endpoints_and_expired_deadlines_stop_before_credential_access() {
    for mode in ["run", "phase", "process", "account", "endpoint", "deadline"] {
        let temp = tempfile::tempdir().unwrap();
        let (store, refs, plan) = fixture(temp.path());
        let memory = Memory::default();
        let mut transaction = store.transaction(until()).unwrap();
        let instance = &mut transaction.registry.instances[0];
        match mode {
            "run" => instance.run_id = "other-run".into(),
            "phase" => instance.state = "starting".into(),
            "process" => instance.process_id = Some(999999),
            "account" => instance.account = Some("10001".into()),
            "endpoint" => instance.endpoint = "http://127.0.0.1:19004/".into(),
            _ => (),
        };
        transaction.commit().unwrap();
        let deadline = if mode == "deadline" {
            Instant::now() - Duration::from_millis(1)
        } else {
            until()
        };
        let error = runtime_store::prepare(&store, &refs, &memory, &plan, deadline)
            .err()
            .unwrap();
        assert_eq!(
            error.code,
            if mode == "deadline" {
                "DEADLINE"
            } else {
                "RUNTIME_PREPARATION_CONFLICT"
            }
        );
        assert_eq!(memory.gets.get(), 0);
        assert_eq!(memory.sets.get(), 0);
        assert!(!store.root().join("runtimes").exists());
    }
}

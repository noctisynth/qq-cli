#![cfg(windows)]
#[path = "common/login_service.rs"]
mod service;
use qq_cli::{
    napcat::runtime::{
        ports::Lease,
        run_job,
        runtime_store::{self, Plan},
        start_intent::{self, Begin, Target},
    },
    native::job::SpawnSpec,
    storage::{
        config::{Config, read_json},
        credentials::{Credentials, SystemCredentials},
        references::References,
        state::{Installation, Instance, Store},
    },
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::Path,
    time::{Duration, Instant},
};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn wait(path: &Path) {
    let end = deadline();
    while !path.exists() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(path.exists(), "original service did not become ready");
}
struct Fixture {
    temp: tempfile::TempDir,
    store: Store,
    credentials: SystemCredentials,
    keys: Vec<String>,
    process: Option<qq_cli::native::process::Pinned>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::write(self.temp.path().join("release"), b"original release");
        if let Some(process) = &self.process {
            let end = deadline();
            while Instant::now() < end {
                if process
                    .verify(end)
                    .is_err_and(|e| e.code == "PROCESS_EXITED")
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        for key in &self.keys {
            let _ = self.credentials.delete(key);
        }
    }
}
impl Fixture {
    fn new(test: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let credentials = SystemCredentials::new(store.root()).unwrap();
        let key = runtime_store::api_credential_key("original-instance", "original-run").unwrap();
        // Establish cleanup before creating any credentials or child process.
        let mut fixture = Self {
            temp,
            store,
            credentials,
            keys: vec![
                key.clone(),
                key.replacen("runtime-api-", "runtime-bundle-", 1),
                "reference-signing-key".into(),
            ],
            process: None,
        };
        let root = fixture.temp.path();
        let store = &fixture.store;
        let credentials = &fixture.credentials;
        let refs = References::open(store, credentials, deadline()).unwrap();
        let lease = Lease::dynamic(deadline()).unwrap();
        let ports = lease.ports();
        let plan = Plan {
            instance_id: "original-instance".into(),
            run_id: "original-run".into(),
            version: "v4.18.38".into(),
            ports,
        };
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.installations.push(Installation {
            id: "original-installation".into(),
            version: plan.version.clone(),
            digest: "a".repeat(64),
            directory: root.join("original-unused-installation"),
        });
        tx.registry.instances.push(Instance {
            id: plan.instance_id.clone(),
            run_id: plan.run_id.clone(),
            installation_id: "original-installation".into(),
            endpoint: format!("http://127.0.0.1:{}/", ports.http),
            ws_endpoint: Some(format!("ws://127.0.0.1:{}/event", ports.events)),
            credential_key: key.clone(),
            account: None,
            state: "preparing".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let prepared =
            runtime_store::prepare(store, &refs, credentials, &plan, deadline()).unwrap();
        std::fs::write(root.join("service.json"),serde_json::to_vec(&json!({"api":format!("127.0.0.1:{}",ports.http),"control":format!("127.0.0.1:{}",ports.control),"apiToken":prepared.bundle().api_token(),"hash":format!("{:x}",Sha256::digest(format!("{}.napcat",prepared.bundle().control_token())))})).unwrap()).unwrap();
        std::fs::write(
            root.join("config.json"),
            serde_json::to_vec(&Config {
                state_dir: Some(store.root().to_owned()),
                ..Default::default()
            })
            .unwrap(),
        )
        .unwrap();
        let target = Target {
            instance_id: plan.instance_id,
            run_id: plan.run_id,
            installation_id: "original-installation".into(),
            installation_digest: "a".repeat(64),
            input_hash: "b".repeat(64),
        };
        let mut environment =
            BTreeMap::from([(OsString::from(service::MODE), root.as_os_str().to_owned())]);
        if let Some(value) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), value);
        }
        let exe = std::env::current_exe().unwrap();
        let args = ["--exact".into(), test.into(), "--test-threads=1".into()];
        let spec = SpawnSpec {
            program: &exe,
            arguments: &args,
            environment: &environment,
            directory: root,
        };
        let Begin::Fresh(intent) =
            start_intent::begin(store, &refs, "original-start", &target, deadline()).unwrap()
        else {
            panic!("fresh original intent expected");
        };
        drop(lease);
        let mut running = None;
        intent
            .dispatch(|tx| {
                running = Some(run_job::launch_for_service_once(
                    tx,
                    &refs,
                    "original-start",
                    &target,
                    &spec,
                    &run_job::ServiceExpectation {
                        image: &exe,
                        control_endpoint: &format!("http://127.0.0.1:{}/", ports.control),
                        ports,
                    },
                    deadline(),
                )?);
                Ok(())
            })
            .unwrap();
        let running = running.unwrap();
        fixture.process = Some(
            qq_cli::native::process::Pinned::reopen(running.root().identity(), deadline()).unwrap(),
        );
        wait(&root.join("service-ready"));
        let mut tx = store.transaction(deadline()).unwrap();
        let capability =
            run_job::retain_prelogin(&tx, &refs, &target, running.job(), deadline()).unwrap();
        tx.registry.instances[0].state = "login_required".into();
        tx.commit().unwrap();
        drop(capability);
        drop(running);
        fixture
    }
    fn status(&self) -> (i32, Value) {
        use std::process::Stdio;
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(["status", "--config"])
            .arg(self.temp.path().join("config.json"))
            .env_remove("QQ_CLI_CONFIG")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let end = deadline();
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= end {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("original status CLI exceeded test deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert!(output.stderr.is_empty());
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert_eq!(text.lines().count(), 1);
        for private in [
            "private-",
            "original-control-credential",
            self.temp.path().to_str().unwrap(),
        ] {
            assert!(!text.contains(private));
        }
        let value: Value = serde_json::from_str(text).unwrap();
        let descriptor = qq_cli::cli::registry::get("status").unwrap();
        let validator = jsonschema::validator_for(&descriptor["responseSchema"]).unwrap();
        assert!(
            validator.is_valid(&value),
            "status output violated schema: {value}"
        );
        (output.status.code().unwrap(), value)
    }
    fn counts(&self) -> Value {
        read_json(&self.temp.path().join("counts.json"), 4096).unwrap()
    }
    fn mode(&self, value: &str) {
        std::fs::write(self.temp.path().join("mode"), value).unwrap();
    }
}

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

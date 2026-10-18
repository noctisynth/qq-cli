use super::service;
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
pub(crate) fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
pub(crate) fn wait(path: &Path) {
    let end = deadline();
    while !path.exists() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(path.exists(), "original service did not become ready");
}
pub(crate) struct Fixture {
    pub(crate) temp: tempfile::TempDir,
    pub(crate) store: Store,
    pub(crate) credentials: SystemCredentials,
    pub(crate) keys: Vec<String>,
    pub(crate) process: Option<qq_cli::native::process::Pinned>,
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
    pub(crate) fn new(test: &str) -> Self {
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
    pub(crate) fn status(&self) -> (i32, Value) {
        self.run("status", &[])
    }
    pub(crate) fn run(&self, command: &str, args: &[&str]) -> (i32, Value) {
        self.finish(command, self.spawn(command, args))
    }
    pub(crate) fn spawn(&self, command: &str, args: &[&str]) -> std::process::Child {
        use std::process::Stdio;
        std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(command.split('.'))
            .args(args)
            .arg("--config")
            .arg(self.temp.path().join("config.json"))
            .env_remove("QQ_CLI_CONFIG")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }
    pub(crate) fn finish(&self, command: &str, mut child: std::process::Child) -> (i32, Value) {
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
        let descriptor = qq_cli::cli::registry::get(command).unwrap();
        let validator = jsonschema::validator_for(&descriptor["responseSchema"]).unwrap();
        assert!(
            validator.is_valid(&value),
            "{command} output violated schema: {value}"
        );
        (output.status.code().unwrap(), value)
    }
    pub(crate) fn counts(&self) -> Value {
        read_json(&self.temp.path().join("counts.json"), 4096).unwrap()
    }
    pub(crate) fn mode(&self, value: &str) {
        std::fs::write(self.temp.path().join("mode"), value).unwrap();
    }
}

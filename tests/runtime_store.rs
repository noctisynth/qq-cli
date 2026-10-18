#![cfg(windows)]
use qq_cli::{
    credentials::{Credentials, SystemCredentials},
    error::{AppError, Result},
    references::References,
    runtime_config::Ports,
    runtime_store::{self, Plan},
    state::{Installation, Instance, Store},
};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn until() -> Instant {
    Instant::now() + Duration::from_secs(10)
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

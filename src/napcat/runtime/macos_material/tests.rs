use super::super::runtime_config::Ports;
use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    time::Duration,
};
#[derive(Default)]
struct Memory {
    values: RefCell<BTreeMap<String, String>>,
    writes: Cell<usize>,
    fail: Cell<usize>,
    panic: Cell<bool>,
    hook: RefCell<Option<Box<dyn Fn()>>>,
}
impl Credentials for Memory {
    fn get(&self, key: &str) -> Result<String> {
        self.values
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::new("CREDENTIAL_NOT_FOUND", "Synthetic absent key", 4))
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.writes.set(self.writes.get() + 1);
        self.values.borrow_mut().insert(key.into(), value.into());
        if let Some(hook) = self.hook.borrow_mut().take() {
            hook();
        }
        if self.writes.get() == self.fail.get() {
            if self.panic.get() {
                panic!("synthetic interrupted credential write");
            }
            return Err(invalid());
        }
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no deletion authorized")
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(20)
}
fn setup() -> (tempfile::TempDir, Store, References, Plan, Memory) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let store = Store::new(base.join("state"));
    let release = crate::napcat::install::catalog::lookup("4.18.41").unwrap();
    let plan = Plan {
        instance_id: "instance".into(),
        run_id: "run".into(),
        version: release.version.into(),
        ports: Ports {
            http: 24001,
            events: 24002,
            control: 24003,
        },
    };
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations.push(Installation {
        id: "install".into(),
        version: release.version.into(),
        digest: release.archive.sha256.into(),
        directory: base.join("framework"),
    });
    tx.registry.instances.push(Instance {
        id: plan.instance_id.clone(),
        run_id: plan.run_id.clone(),
        installation_id: "install".into(),
        endpoint: "http://127.0.0.1:24001/".into(),
        ws_endpoint: Some("ws://127.0.0.1:24002/event".into()),
        credential_key: runtime_store::api_credential_key("instance", "run").unwrap(),
        account: None,
        state: "preparing".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let refs = References::with_key(store.root().join("references"), [17; 32]);
    assert!(matches!(
        macos_layout::prepare(&store, &refs, "instance", "layout", deadline()).unwrap(),
        macos_layout::Preparation::Ready(_)
    ));
    (temp, store, refs, plan, Memory::default())
}
#[test]
fn complete_promotes_once_and_recovers_original_material() {
    let (_temp, store, refs, plan, memory) = setup();
    let Preparation::Ready(first) =
        prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap()
    else {
        panic!("not ready")
    };
    assert_eq!(memory.writes.get(), 2);
    assert_eq!(store.read().unwrap().instances[0].state, "prepared");
    let Some(Preparation::Ready(second)) =
        recover(&store, &refs, &memory, &plan, deadline()).unwrap()
    else {
        panic!("not ready")
    };
    assert_eq!(first.bundle().api_token(), second.bundle().api_token());
    use std::os::unix::fs::MetadataExt;
    let registry_inode = std::fs::metadata(store.root().join("registry.json"))
        .unwrap()
        .ino();
    assert!(matches!(
        prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap(),
        Preparation::Ready(_)
    ));
    assert_eq!(
        std::fs::metadata(store.root().join("registry.json"))
            .unwrap()
            .ino(),
        registry_inode
    );
    assert!(prepare(&store, &refs, &memory, &plan, "other", deadline()).is_err());
    assert_eq!(memory.writes.get(), 2);
    let record = load(&path(&store, &plan).unwrap(), &refs).unwrap().unwrap();
    let public = serde_json::to_string(&record).unwrap();
    for secret in [
        first.bundle().api_token(),
        first.bundle().control_token(),
        first.bundle().jwt_secret(),
    ] {
        assert!(!public.contains(secret));
    }
    // Completion before state commit is recoverable without repeating effects.
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "preparing".into();
    tx.commit().unwrap();
    assert!(matches!(
        recover(&store, &refs, &memory, &plan, deadline()).unwrap(),
        Some(Preparation::Ready(_))
    ));
    assert_eq!(store.read().unwrap().instances[0].state, "preparing");
    prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap();
    assert_eq!(store.read().unwrap().instances[0].state, "prepared");
    assert_eq!(memory.writes.get(), 2);
}
#[test]
fn missing_or_changed_secrets_and_config_never_regenerate() {
    for change in 0..3 {
        let (_temp, store, refs, plan, memory) = setup();
        let Preparation::Ready(first) =
            prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap()
        else {
            panic!("not ready")
        };
        let key = runtime_store::api_credential_key("instance", "run").unwrap();
        match change {
            0 => {
                memory.values.borrow_mut().remove(&key);
            }
            1 => {
                memory.values.borrow_mut().insert(key, "different".into());
            }
            _ => std::fs::write(
                first.directory().join("napcat-data/config/webui.json"),
                b"changed",
            )
            .unwrap(),
        }
        assert!(recover(&store, &refs, &memory, &plan, deadline()).is_err());
        assert!(prepare(&store, &refs, &memory, &plan, "request", deadline()).is_err());
        assert_eq!(memory.writes.get(), 2);
    }
}
#[test]
fn credential_failure_and_panic_never_replay_or_promote() {
    for panic in [false, true] {
        for fail in [1, 2] {
            let (_temp, store, refs, plan, memory) = setup();
            memory.fail.set(fail);
            memory.panic.set(panic);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prepare(&store, &refs, &memory, &plan, "request", deadline())
            }));
            if panic {
                assert!(result.is_err());
            } else {
                assert!(matches!(result.unwrap().unwrap(), Preparation::Unknown));
            }
            assert!(matches!(
                recover(&store, &refs, &memory, &plan, deadline()).unwrap(),
                Some(Preparation::Unknown)
            ));
            assert!(matches!(
                prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap(),
                Preparation::Unknown
            ));
            assert!(prepare(&store, &refs, &memory, &plan, "other", deadline()).is_err());
            assert_eq!(memory.writes.get(), fail);
            assert_eq!(store.read().unwrap().instances[0].state, "preparing");
        }
    }
}
#[test]
fn foreign_keys_and_changed_scope_reject_before_writes() {
    for change in 0..4 {
        let (_temp, store, refs, mut plan, memory) = setup();
        match change {
            0 => {
                memory.values.borrow_mut().insert(
                    runtime_store::api_credential_key("instance", "run").unwrap(),
                    "preserve".into(),
                );
            }
            1 => {
                plan.ports.control = plan.ports.http;
            }
            2 => {
                plan.run_id = "different".into();
            }
            _ => {
                let mut tx = store.transaction(deadline()).unwrap();
                tx.registry.instances[0].state = "ready".into();
                tx.commit().unwrap();
            }
        }
        assert!(prepare(&store, &refs, &memory, &plan, "request", deadline()).is_err());
        assert_eq!(memory.writes.get(), 0);
        assert!(!store.root().join("macos-materials").exists());
    }
}

#[test]
fn state_change_or_lock_replacement_during_vault_write_retains_pending() {
    for replace in [false, true] {
        let (_temp, store, refs, plan, memory) = setup();
        let root = store.root().to_owned();
        *memory.hook.borrow_mut() = Some(Box::new(move || {
            if replace {
                std::fs::rename(root.join("state.lock"), root.join("old.lock")).unwrap();
                std::fs::write(root.join("state.lock"), b"").unwrap();
            } else {
                let other = Store::new(root.clone());
                let mut tx = other.transaction(deadline()).unwrap();
                tx.registry.instances[0].endpoint = "http://127.0.0.1:24009/".into();
                tx.commit().unwrap();
            }
        }));
        let result = prepare(&store, &refs, &memory, &plan, "request", deadline());
        assert_eq!(result.err().unwrap().code, "MACOS_MATERIAL_OUTCOME_UNKNOWN");
        let record = load(&path(&store, &plan).unwrap(), &refs).unwrap().unwrap();
        assert!(matches!(record.outcome, Outcome::Pending));
        assert_eq!(memory.writes.get(), 1);
        assert_eq!(store.read().unwrap().instances[0].state, "preparing");
        // Even when current scope is restored, the original claim cannot repeat.
        if !replace {
            let mut tx = store.transaction(deadline()).unwrap();
            tx.registry.instances[0].endpoint = "http://127.0.0.1:24001/".into();
            tx.commit().unwrap();
        }
        assert!(matches!(
            prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap(),
            Preparation::Unknown
        ));
        assert_eq!(memory.writes.get(), 1);
    }
}

#[test]
fn configuration_conflict_after_pending_preserves_keys_and_unknown_outcome() {
    let (_temp, store, refs, plan, memory) = setup();
    let config = store
        .root()
        .join("runtimes")
        .join(runtime_store::identity("instance", "run").unwrap())
        .join("napcat-data/config");
    let path = config.join("foreign.json");
    let injected = path.clone();
    *memory.hook.borrow_mut() = Some(Box::new(move || {
        std::fs::write(&injected, b"preserve").unwrap()
    }));
    assert!(matches!(
        prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap(),
        Preparation::Unknown
    ));
    assert_eq!(memory.writes.get(), 2);
    assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
    assert_eq!(std::fs::read_dir(&config).unwrap().count(), 1);
    assert!(matches!(
        prepare(&store, &refs, &memory, &plan, "request", deadline()).unwrap(),
        Preparation::Unknown
    ));
    assert_eq!(memory.writes.get(), 2);
    assert_eq!(store.read().unwrap().instances[0].state, "preparing");
}

//! Signed d2/s1/j2/p2 composition with an original synthetic application.
use super::*;
use crate::{
    napcat::{
        install::macos::{
            bundle::{BundleRole, Inventory},
            signing::Policy,
        },
        runtime::{
            macos::{
                compatibility::Key,
                deploy::{self, Status},
                run,
            },
            start_intent::{self, Begin, Outcome},
        },
    },
    storage::{
        references::References,
        state::{Installation, Instance, Store, private_dir},
    },
};

struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    refs: References,
    target: deploy::Target,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
        let refs = References::with_key(store.root().join("references"), [74; 32]);
        let installation = Installation {
            id: "install".into(),
            version: "synthetic".into(),
            digest: "a".repeat(64),
            directory: store.root().join("framework"),
        };
        let instance = Instance {
            id: "instance".into(),
            run_id: "run".into(),
            installation_id: installation.id.clone(),
            endpoint: "http://127.0.0.1:24001/".into(),
            ws_endpoint: None,
            credential_key: "synthetic".into(),
            account: None,
            state: "prepared".into(),
            process_id: None,
            process_started: None,
        };
        let target = deploy::Target {
            instance_id: instance.id.clone(),
            run_id: instance.run_id.clone(),
            installation_id: installation.id.clone(),
            installation_digest: installation.digest.clone(),
            input_hash: "b".repeat(64),
        };
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances.push(instance);
        tx.registry.installations.push(installation.clone());
        tx.commit().unwrap();
        Self {
            _temp: temp,
            store,
            refs,
            target,
        }
    }
    fn instance(&self) -> Instance {
        self.store.read().unwrap().instances[0].clone()
    }
    fn root(&self) -> PathBuf {
        let id = crate::napcat::runtime::runtime_store::identity("instance", "run").unwrap();
        self.store.root().join("runtimes").join(id).join("QQ.app")
    }
    fn launch(&self, program: &Program, tree: bool) -> Owned {
        let inventory = Inventory::inspect(&program.root, deadline()).unwrap();
        assert_eq!(
            deploy::deploy(
                &self.store,
                &self.refs,
                "deployment",
                &self.target,
                deadline(),
                |root, guard| {
                    private_dir(root.parent().unwrap())?;
                    let copy = inventory
                        .copy_to_new(root.parent().unwrap(), "QQ.app", deadline(), guard)?
                        .harden_copy(deadline(), guard)?;
                    let plan = copy.signing_plan(
                        &[BundleRole::new(
                            std::path::Path::new(""),
                            "org.qq-cli.stop-fixture",
                            "QQ",
                            Policy::Main,
                        )?],
                        deadline(),
                    )?;
                    copy.sign_copy(&plan, deadline(), guard)
                }
            )
            .unwrap(),
            Status::Complete
        );
        let app = self.root();
        let profile = format!(
            "(version 1)(allow default)(deny process-exec)(allow process-exec (subpath {}))",
            serde_json::to_string(&app).unwrap()
        );
        let scope = run::Scope::declare(app.clone(), &profile);
        let mut target = self.target.clone();
        target.input_hash = "c".repeat(64);
        let Begin::Fresh(intent) =
            start_intent::begin(&self.store, &self.refs, "launch", &target, deadline()).unwrap()
        else {
            panic!("fresh launch")
        };
        let mut owned = None;
        let record = intent
            .dispatch(|tx| {
                let mut command = Command::new("/usr/bin/sandbox-exec");
                command
                    .args(["-p", &profile])
                    .arg(app.join("Contents/MacOS/QQ"))
                    .arg(if tree { "tree" } else { "leaf" });
                let child = Program::spawn_command(command);
                let original = child.target.reopen(deadline())?;
                run::publish(
                    tx,
                    &self.refs,
                    "launch",
                    &original,
                    run::Binding {
                        target: &target,
                        compatibility: &Key::current()?,
                        scope: &scope,
                        control_endpoint: "http://127.0.0.1:24003/",
                    },
                    deadline(),
                )?;
                owned = Some(child);
                Ok(())
            })
            .unwrap();
        assert_eq!(record.outcome, Outcome::Dispatched);
        owned.unwrap()
    }
    fn stop(&self) -> Result<Stopped> {
        execute(&self.store, &self.refs, &self.instance(), deadline())
    }
}

#[test]
fn signed_launch_stop_and_repeated_stop_share_one_completion_proof() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, true);
    let result = fixture.stop().unwrap();
    assert_eq!(result.instance_id, "instance");
    assert!(!result.reused);
    assert_eq!(fixture.instance().state, "stopped");
    assert!(owned.child.wait().unwrap().code().is_none());
    let complete = journal::load(&fixture.store, &fixture.refs, &fixture.instance())
        .unwrap()
        .unwrap();
    assert!(complete.completed);
    assert_eq!(complete.members.len(), 2);
    assert!(fixture.stop().unwrap().reused);
    assert!(
        journal::load(&fixture.store, &fixture.refs, &fixture.instance())
            .unwrap()
            .unwrap()
            == complete
    );
    assert!(process::exited(owned.descendant.as_ref().unwrap().identity(), deadline()).unwrap());
}

#[test]
fn stopped_synthetic_runtime_can_be_unregistered_without_deleting_its_material() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, true);
    fixture.stop().unwrap();
    owned.child.wait().unwrap();
    let inventory = Inventory::inspect(&fixture.root(), deadline()).unwrap();
    let original_installations = fixture.store.read().unwrap().installations;
    let result = crate::napcat::runtime::remove::execute(
        &fixture.store,
        &fixture.refs,
        "instance",
        deadline(),
    )
    .unwrap();
    assert!(!result.reused);
    assert!(fixture.store.read().unwrap().instances.is_empty());
    assert_eq!(
        fixture.store.read().unwrap().installations,
        original_installations
    );
    inventory.revalidate(deadline()).unwrap();
    assert!(fixture.store.root().join("macos-runs").exists());
    assert!(fixture.store.root().join("macos-stops").exists());
    assert!(
        crate::napcat::runtime::remove::execute(
            &fixture.store,
            &fixture.refs,
            "instance",
            deadline()
        )
        .unwrap()
        .reused
    );
    inventory.revalidate(deadline()).unwrap();
}

#[test]
fn synthetic_owned_run_stop_and_successor_claim_clear_previous_identity() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, true);
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.registry.instances[0].account = Some("10001".into());
    tx.registry.instances[0].process_id = Some(owned.child.id());
    tx.registry.instances[0].process_started = Some("synthetic-observation".into());
    tx.commit().unwrap();
    fixture.stop().unwrap();
    owned.child.wait().unwrap();
    let source = fixture.instance();
    let original = Inventory::inspect(&fixture.root(), deadline()).unwrap();
    let lifecycle = crate::napcat::runtime::lifecycle_lock(&fixture.store, deadline()).unwrap();
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    let installed = tx.registry.installations[0].clone();
    let next = crate::napcat::runtime::generation::reserve_macos(
        &mut tx,
        &fixture.refs,
        &source,
        &installed,
        &lifecycle,
        deadline(),
    )
    .unwrap();
    let current = tx.registry.instances[0].clone();
    assert_eq!(current.id, source.id);
    assert_ne!(current.run_id, source.run_id);
    assert!(
        current.account.is_none()
            && current.process_id.is_none()
            && current.process_started.is_none()
    );
    assert_ne!(current.credential_key, source.credential_key);
    assert_eq!(current.state, "preparing");
    drop(next);
    drop(tx);
    drop(lifecycle);
    original.revalidate(deadline()).unwrap();
}

#[test]
fn production_restart_rejects_unavailable_target_before_affecting_synthetic_run() {
    struct NoVault;
    impl crate::storage::credentials::Credentials for NoVault {
        fn get(&self, _: &str) -> Result<String> {
            panic!("no vault access")
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            panic!("no vault write")
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("no vault deletion")
        }
    }
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, false);
    let selected = fixture.instance();
    let context = crate::napcat::runtime::macos::start::Context {
        store: &fixture.store,
        references: &fixture.refs,
        credentials: &NoVault,
    };
    assert_eq!(
        crate::napcat::runtime::macos::restart::execute(&context, &selected, deadline())
            .err()
            .unwrap()
            .code,
        "VERSION_UNSUPPORTED"
    );
    assert_eq!(fixture.instance(), selected);
    assert!(owned.child.try_wait().unwrap().is_none());
    assert!(
        !owned
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
    assert!(!fixture.store.root().join("macos-stops").exists());
    assert!(!fixture.store.root().join("run-transitions").exists());
}
#[test]
fn stopped_acknowledgement_recovers_without_requiring_the_app_or_repeating_signals() {
    let fixture = Fixture::new();
    let program = Program::new();
    let _owned = fixture.launch(&program, false);
    fixture.stop().unwrap();
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "stopping".into();
    tx.commit().unwrap();
    std::fs::remove_dir_all(fixture.root()).unwrap();
    assert!(fixture.stop().unwrap().reused);
    assert_eq!(fixture.instance().state, "stopped");
}
#[test]
fn undispatched_preparation_stops_but_a_run_claim_without_j2_does_not() {
    let fixture = Fixture::new();
    assert!(!fixture.stop().unwrap().reused);
    assert!(fixture.stop().unwrap().reused);
    let fixture = Fixture::new();
    let Begin::Fresh(intent) = start_intent::begin(
        &fixture.store,
        &fixture.refs,
        "launch",
        &fixture.target,
        deadline(),
    )
    .unwrap() else {
        panic!("fresh")
    };
    drop(intent);
    assert_eq!(fixture.stop().err().unwrap().code, "STOP_UNVERIFIABLE");
    assert_eq!(fixture.instance().state, "starting");
    assert!(!fixture.store.root().join("macos-stops").exists());
}
#[test]
fn unknown_foreign_candidate_is_preserved_then_the_original_stop_can_resume() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, false);
    let foreign = Program::spawn_command(Command::new(fixture.root().join("Contents/MacOS/QQ")));
    assert_eq!(fixture.stop().err().unwrap().code, "STOP_OUTCOME_UNKNOWN");
    assert_eq!(fixture.instance().state, "stopping");
    assert!(
        !foreign
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
    let pending = journal::load(&fixture.store, &fixture.refs, &fixture.instance())
        .unwrap()
        .unwrap();
    assert!(!pending.completed);
    assert_eq!(pending.members.len(), 1);
    drop(foreign);
    fixture.stop().unwrap();
    assert_eq!(fixture.instance().state, "stopped");
    owned.child.wait().unwrap();
}
#[test]
fn changed_installation_or_foreign_signing_key_rejects_before_stopping() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, false);
    let wrong = References::with_key(fixture.store.root().join("references"), [75; 32]);
    assert!(execute(&fixture.store, &wrong, &fixture.instance(), deadline()).is_err());
    assert!(owned.child.try_wait().unwrap().is_none());
    assert!(
        !owned
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.installations[0].digest = "d".repeat(64);
    tx.commit().unwrap();
    assert!(fixture.stop().is_err());
    assert!(owned.child.try_wait().unwrap().is_none());
    assert!(
        !owned
            .target
            .reopen(deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
}

fn rewrite(
    fixture: &Fixture,
    kind: &str,
    path: &std::path::Path,
    edit: impl FnOnce(&mut serde_json::Value),
) {
    let mut pointer: serde_json::Value = crate::storage::config::read_json(path, 4096).unwrap();
    let mut record: serde_json::Value = fixture
        .refs
        .get(kind, pointer["reference"].as_str().unwrap())
        .unwrap();
    edit(&mut record);
    pointer["reference"] = fixture.refs.issue(kind, &record).unwrap().into();
    crate::storage::state::atomic_json(path, &pointer).unwrap();
}
fn run_index(fixture: &Fixture) -> PathBuf {
    use sha2::{Digest, Sha256};
    fixture.store.root().join("macos-runs").join(format!(
        "{:x}.json",
        Sha256::digest(serde_json::to_vec(&("instance", "run")).unwrap())
    ))
}

#[test]
fn authenticated_j2_can_stop_an_unknown_original_dispatch_without_replaying_it() {
    use sha2::{Digest, Sha256};
    for outcome in ["pending", "unknown"] {
        let fixture = Fixture::new();
        let program = Program::new();
        let mut owned = fixture.launch(&program, false);
        let request = fixture
            .store
            .root()
            .join("start-requests")
            .join(format!("{:x}.json", Sha256::digest(b"launch")));
        rewrite(&fixture, "s1", &request, |record| {
            record["outcome"] = outcome.into()
        });
        fixture.stop().unwrap();
        assert_eq!(fixture.instance().state, "stopped");
        owned.child.wait().unwrap();
        assert_eq!(
            start_intent::get(&fixture.store, &fixture.refs, "launch")
                .unwrap()
                .outcome,
            if outcome == "pending" {
                Outcome::Pending
            } else {
                Outcome::Unknown
            }
        );
    }
}
#[test]
fn legacy_j2_without_original_scope_or_audit_target_never_grants_stop_authority() {
    for field in ["scope", "signal"] {
        let fixture = Fixture::new();
        let program = Program::new();
        let mut owned = fixture.launch(&program, false);
        rewrite(&fixture, "j2", &run_index(&fixture), |record| {
            record.as_object_mut().unwrap().remove(field);
        });
        assert_eq!(fixture.stop().err().unwrap().code, "STOP_UNVERIFIABLE");
        assert!(owned.child.try_wait().unwrap().is_none());
        assert!(
            !owned
                .target
                .reopen(deadline())
                .unwrap()
                .is_suspended(deadline())
                .unwrap()
        );
        assert!(!fixture.store.root().join("macos-stops").exists());
    }
}
#[test]
fn changed_boot_completes_only_the_old_record_without_signalling_a_current_pid() {
    let fixture = Fixture::new();
    let program = Program::new();
    let mut owned = fixture.launch(&program, false);
    rewrite(&fixture, "j2", &run_index(&fixture), |record| {
        // Same current PID and executable, but an explicitly different original
        // boot. The signed fixture models an old run, never a live replacement.
        for field in ["owner", "signal"] {
            let identity = if field == "owner" {
                &mut record[field]
            } else {
                &mut record[field]["identity"]
            };
            identity["bootSession"] = "00000000-0000-0000-0000-000000000001".into();
        }
    });
    std::fs::rename(fixture.root(), fixture.root().with_extension("retired")).unwrap();
    fixture.stop().unwrap();
    assert_eq!(fixture.instance().state, "stopped");
    assert!(owned.child.try_wait().unwrap().is_none());
    assert!(
        !Pinned::observe(owned.child.id(), deadline())
            .unwrap()
            .is_suspended(deadline())
            .unwrap()
    );
}

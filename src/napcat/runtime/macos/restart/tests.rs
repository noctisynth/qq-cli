use super::*;
use crate::napcat::install::catalog;
use crate::{
    napcat::runtime::{
        instance_creation::{Admission, Creation},
        macos::{layout, material},
        runtime_store::Plan,
    },
    storage::{references::References, state::Store},
};
use std::{cell::RefCell, collections::BTreeMap, time::Duration};

#[derive(Default)]
struct Memory(RefCell<BTreeMap<String, String>>);
impl Credentials for Memory {
    fn get(&self, key: &str) -> Result<String> {
        self.0
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::new("CREDENTIAL_NOT_FOUND", "Synthetic material missing", 4))
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.0.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no credential deletion")
    }
}
fn end() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn setup() -> (tempfile::TempDir, Store, References, Memory, Plan) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [97; 32]);
    let release = catalog::lookup("4.18.41").unwrap();
    let installed = Installation {
        id: "install".into(),
        version: release.version.into(),
        digest: release.archive.sha256.into(),
        directory: store.root().join("framework"),
    };
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.installations.push(installed.clone());
    tx.commit().unwrap();
    // Protocol fixture: production preflight still requires the real i1 and
    // pinned source. These tests never turn this metadata into a QQ launch.
    let Creation::Fresh(reservation) = instance_creation::reserve_in(
        store.transaction(end()).unwrap(),
        Admission {
            refs: &refs,
            request_id: "original",
            release: &release,
            installation: installed,
            kind: "n2",
            guard: &|| Ok(()),
        },
        end(),
    )
    .unwrap() else {
        panic!("fresh")
    };
    let plan = reservation.plan().clone();
    drop(reservation);
    (temp, store, refs, Memory::default(), plan)
}
fn instance(store: &Store) -> Instance {
    store.read().unwrap().instances[0].clone()
}
fn prepared(
    context: &start::Context<'_, Memory>,
    request: &str,
    plan: &Plan,
) -> material::Prepared {
    assert!(matches!(
        layout::prepare(
            context.store,
            context.references,
            &plan.instance_id,
            request,
            end()
        )
        .unwrap(),
        layout::Preparation::Ready(_)
    ));
    let material::Preparation::Ready(material) = material::prepare(
        context.store,
        context.references,
        context.credentials,
        plan,
        request,
        end(),
    )
    .unwrap() else {
        panic!("material")
    };
    material
}
fn result(instance: Instance, request: &str) -> start::Started {
    start::Started {
        instance,
        reused: false,
        request: Some(request.into()),
    }
}

#[test]
fn stopped_restart_prepares_one_successor_with_fresh_secrets_and_keeps_original_material() {
    let (_temp, store, refs, memory, plan) = setup();
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    let original = prepared(&context, "original", &plan);
    let profile_marker = original.profile_home().join("synthetic-session");
    std::fs::write(&profile_marker, b"owned synthetic session").unwrap();
    let old_secrets = memory.0.borrow().clone();
    stop::execute(&store, &refs, &instance(&store), end()).unwrap();
    let source = instance(&store);
    let first = execute_with(
        &context,
        &source,
        end(),
        |_, _, _, _| Ok(()),
        |context, current, request, _| {
            let checkpoint = super::super::creation::checkpoint(&store, &refs, request)?.unwrap();
            let next = prepared(context, request, &checkpoint.plan);
            assert_eq!(next.profile_home(), original.profile_home());
            assert_ne!(next.directory(), original.directory());
            assert_eq!(
                std::fs::read(&profile_marker).unwrap(),
                b"owned synthetic session"
            );
            assert_eq!(current.id, source.id);
            assert_ne!(current.run_id, source.run_id);
            assert!(current.account.is_none() && current.process_id.is_none());
            assert!(original.bundle().api_token() != next.bundle().api_token());
            assert!(original.bundle().jwt_secret() != next.bundle().jwt_secret());
            assert!(original.bundle().control_token() != next.bundle().control_token());
            assert!(
                old_secrets
                    .iter()
                    .all(|(key, value)| memory.0.borrow().get(key) == Some(value))
            );
            Ok(result(instance(&store), request))
        },
    )
    .unwrap();
    let request = first.request.clone().unwrap();
    let current = instance(&store);
    let again = execute_with(
        &context,
        &source,
        end(),
        |_, _, _, _| panic!("claimed successor must not preflight or stop the old run again"),
        |_, seen, key, _| {
            assert_eq!(seen, &current);
            assert_eq!(key, request);
            Ok(result(seen.clone(), key))
        },
    )
    .unwrap();
    assert_eq!(again.instance, current);
    assert_eq!(store.read().unwrap().instances.len(), 1);
    assert_eq!(
        std::fs::read_dir(store.root().join("run-transitions"))
            .unwrap()
            .count(),
        1
    );
    assert!(original.directory().is_dir());
    assert!(!store.root().join("start-requests").exists());
    assert!(!store.root().join("macos-runs").exists());
}

#[test]
fn failed_preflight_preserves_live_registration_without_a_stop_or_generation_intent() {
    let (_temp, store, refs, memory, _) = setup();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.commit().unwrap();
    let source = instance(&store);
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    let error = execute_with(
        &context,
        &source,
        end(),
        |_, _, _, _| {
            Err(AppError::new(
                "SYNTHETIC_PREFLIGHT",
                "Preflight failed before stop",
                4,
            ))
        },
        |_, _, _, _| panic!("no preparation"),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "SYNTHETIC_PREFLIGHT");
    assert_eq!(instance(&store), source);
    assert!(!store.root().join("macos-stops").exists());
    assert!(!store.root().join("run-transitions").exists());
}

#[test]
fn initial_preparation_resumes_its_own_key_without_stopping_or_minting_a_run() {
    let (_temp, store, refs, memory, plan) = setup();
    let source = instance(&store);
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    execute_with(
        &context,
        &source,
        end(),
        |_, _, _, _| panic!("resume must use normal start preflight"),
        |_, seen, key, _| {
            assert_eq!(key, "original");
            assert_eq!(seen.run_id, plan.run_id);
            Ok(result(seen.clone(), key))
        },
    )
    .unwrap();
    assert_eq!(instance(&store), source);
    assert!(!store.root().join("macos-stops").exists());
    assert!(!store.root().join("run-transitions").exists());
}

#[test]
fn changed_registration_during_preflight_is_not_stopped_or_used_as_a_replacement() {
    let (_temp, store, refs, memory, _) = setup();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.commit().unwrap();
    let source = instance(&store);
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    assert!(
        execute_with(
            &context,
            &source,
            end(),
            |_, _, _, _| {
                let mut tx = store.transaction(end())?;
                tx.registry.instances[0].run_id = "replacement".into();
                tx.commit()
            },
            |_, _, _, _| panic!("no preparation")
        )
        .is_err()
    );
    assert_eq!(instance(&store).run_id, "replacement");
    assert!(!store.root().join("macos-stops").exists());
    assert!(!store.root().join("run-transitions").exists());
}

#[test]
fn lost_state_lock_after_preflight_prevents_stop() {
    let (_temp, store, refs, memory, _) = setup();
    let mut tx = store.transaction(end()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.commit().unwrap();
    let source = instance(&store);
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    assert!(
        execute_with(
            &context,
            &source,
            end(),
            |_, _, _, _| {
                std::fs::rename(
                    store.root().join("state.lock"),
                    store.root().join("retired.lock"),
                )
                .unwrap();
                std::fs::write(store.root().join("state.lock"), b"").unwrap();
                Ok(())
            },
            |_, _, _, _| panic!("no preparation")
        )
        .is_err()
    );
    assert_eq!(instance(&store), source);
    assert!(!store.root().join("macos-stops").exists());
}

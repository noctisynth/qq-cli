use super::*;
use crate::{
    napcat::{
        install::catalog,
        runtime::{
            instance_creation::{Admission, Creation},
            macos::release,
        },
    },
    storage::{references::References, state::Store},
};
use std::time::Duration;

struct NoSecrets;
impl Credentials for NoSecrets {
    fn get(&self, _: &str) -> Result<String> {
        panic!("no credential reads")
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("no credential writes")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no credential deletion")
    }
}
fn end() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn setup() -> (
    tempfile::TempDir,
    Store,
    References,
    Instance,
    Installation,
    Installation,
) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [98; 32]);
    let installed = |version: &str| {
        let release = release::lookup(version).unwrap();
        Installation {
            id: format!("install-{}", version.replace('.', "-")),
            version: release.version.into(),
            digest: release.archive.sha256.into(),
            directory: store.root().join(version),
        }
    };
    let original = installed("4.18.38");
    let target = installed("4.18.41");
    let mut tx = store.transaction(end()).unwrap();
    tx.registry
        .installations
        .extend([original.clone(), target.clone()]);
    tx.commit().unwrap();
    // Protocol-only admission: production preflight independently verifies the
    // actual i1/source. The callback below never prepares or launches QQ.
    let creation = instance_creation::reserve_in(
        store.transaction(end()).unwrap(),
        Admission {
            refs: &refs,
            request_id: "original",
            release: &catalog::lookup("4.18.38").unwrap(),
            installation: original.clone(),
            kind: "n2",
            guard: &|| Ok(()),
        },
        end(),
    )
    .unwrap();
    assert!(matches!(creation, Creation::Fresh(_)));
    drop(creation);
    let source = store.read().unwrap().instances.remove(0);
    (temp, store, refs, source, original, target)
}
fn action(target: &Installation, operation: Operation) -> Action<'_> {
    Action {
        installation: target,
        operation,
        trial: operation == Operation::Update,
    }
}
fn resumed(
    _: &start::Context<'_, NoSecrets>,
    current: &Instance,
    target: &Installation,
    request: Option<&str>,
    _: bool,
    _: Instant,
) -> Result<start::Started> {
    assert_eq!(current.installation_id, target.id);
    Ok(start::Started {
        instance: current.clone(),
        reused: false,
        request: request.map(str::to_owned),
    })
}

#[test]
fn update_and_rollback_reuse_original_successors_and_preserve_history() {
    let (_temp, store, refs, source, original, target) = setup();
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    let first = execute_with(
        &context,
        &source,
        action(&target, Operation::Update),
        end(),
        |_, _, _, _, _, _| Ok(()),
        resumed,
    )
    .unwrap();
    assert_eq!(first.instance.id, source.id);
    assert_ne!(first.instance.run_id, source.run_id);
    assert!(first.instance.account.is_none() && first.instance.process_id.is_none());
    // The original default pointer still describes the source version. A
    // signed successor supplies its own creation key rather than replaying it.
    let defaults = store.root().join("automatic-start");
    crate::storage::state::private_dir(&defaults).unwrap();
    let reference = refs.issue("d1", &serde_json::json!({"schemaVersion":1,"requestId":"original","version":original.version})).unwrap();
    crate::storage::state::atomic_json(
        &defaults.join("active.json"),
        &serde_json::json!({"schemaVersion":1,"reference":reference}),
    )
    .unwrap();
    let auto = start::execute(
        &context,
        start::Options {
            instance: None,
            version: None,
            request: None,
            new: false,
            verify_compatibility: false,
        },
        end(),
    )
    .err()
    .unwrap();
    assert_eq!(auto.code, "COMPATIBILITY_UNKNOWN");
    assert_eq!(auto.request_id, first.request);
    assert_eq!(
        generation::rollback_installation(&store, &refs, &first.instance).unwrap(),
        Some(original.clone())
    );
    let again = execute_with(
        &context,
        &source,
        action(&target, Operation::Update),
        end(),
        |_, _, _, _, _, _| panic!("do not stop/preflight an already claimed source"),
        resumed,
    )
    .unwrap();
    assert_eq!(again.instance, first.instance);
    assert_eq!(again.request, first.request);
    assert_eq!(
        version_intent::rollback_target(&store, &refs, &first.instance, end()).unwrap(),
        original
    );
    let rolled = execute_with(
        &context,
        &first.instance,
        action(&original, Operation::Rollback),
        end(),
        |_, _, _, _, _, _| Ok(()),
        resumed,
    )
    .unwrap();
    assert_eq!(rolled.instance.installation_id, original.id);
    assert_eq!(
        generation::rollback_installation(&store, &refs, &rolled.instance).unwrap(),
        Some(target.clone())
    );
    let repeated = execute_with(
        &context,
        &first.instance,
        action(&original, Operation::Rollback),
        end(),
        |_, _, _, _, _, _| panic!("original rollback must not toggle"),
        resumed,
    )
    .unwrap();
    assert_eq!(repeated.instance, rolled.instance);
    assert_eq!(
        version_intent::rollback_target(&store, &refs, &rolled.instance, end()).unwrap(),
        original
    );
    // An ordinary new run inherits the last different installation, not the
    // previous maintenance action. Its old installation cannot be uninstalled.
    stop::execute(&store, &refs, &rolled.instance, end()).unwrap();
    let stopped = store.read().unwrap().instances.remove(0);
    let lifecycle = lifecycle_lock(&store, end()).unwrap();
    let mut tx = store.transaction(end()).unwrap();
    let next =
        generation::reserve_macos(&mut tx, &refs, &stopped, &original, &lifecycle, end()).unwrap();
    drop(next);
    drop(tx);
    drop(lifecycle);
    let current = store.read().unwrap().instances.remove(0);
    assert_eq!(
        generation::maintenance_operation(&store, &refs, &current).unwrap(),
        None
    );
    assert_eq!(
        generation::rollback_installation(&store, &refs, &current).unwrap(),
        Some(target.clone())
    );
    assert_eq!(
        crate::napcat::install::installation::uninstall(&store, &target.id, &refs, end())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
    assert_eq!(store.read().unwrap().installations.len(), 2);
    assert_eq!(
        std::fs::read_dir(store.root().join("instance-creations"))
            .unwrap()
            .count(),
        4
    );
}

#[test]
fn pending_intent_blocks_original_start_restart_and_target_uninstall_then_recovers() {
    let (_temp, store, refs, source, _, target) = setup();
    version_intent::begin(&store, &refs, &source, &target, Operation::Update, end()).unwrap();
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    assert_eq!(
        start::selected(
            &context,
            &source,
            &store.read().unwrap().installations[0],
            false,
            end()
        )
        .err()
        .unwrap()
        .code,
        "VERSION_MAINTENANCE_PENDING"
    );
    assert_eq!(
        super::super::restart::execute(&context, &source, end())
            .err()
            .unwrap()
            .code,
        "VERSION_MAINTENANCE_PENDING"
    );
    assert_eq!(
        crate::napcat::install::installation::uninstall(&store, &target.id, &refs, end())
            .err()
            .unwrap()
            .code,
        "INSTALLATION_IN_USE"
    );
    stop::execute(&store, &refs, &source, end()).unwrap();
    let recovered = execute_with(
        &context,
        &source,
        action(&target, Operation::Update),
        end(),
        |_, _, _, _, _, _| Ok(()),
        resumed,
    )
    .unwrap();
    assert_eq!(recovered.instance.installation_id, target.id);
    assert!(store.read().unwrap().maintenance_intents.is_empty());
    assert!(!store.root().join("start-requests").exists());
}

#[test]
fn failed_preflight_or_replaced_lock_never_publishes_an_intent_or_stops() {
    for replace_lock in [false, true] {
        let (_temp, store, refs, source, _, target) = setup();
        let context = start::Context {
            store: &store,
            references: &refs,
            credentials: &NoSecrets,
        };
        let error = execute_with(
            &context,
            &source,
            action(&target, Operation::Update),
            end(),
            |_, _, _, _, _, _| {
                if replace_lock {
                    std::fs::rename(
                        store.root().join("state.lock"),
                        store.root().join("original.lock"),
                    )
                    .unwrap();
                    std::fs::write(store.root().join("state.lock"), b"").unwrap();
                    Ok(())
                } else {
                    Err(AppError::new(
                        "SYNTHETIC_PREFLIGHT",
                        "No actual preparation",
                        4,
                    ))
                }
            },
            |_, _, _, _, _, _| panic!("no resume"),
        )
        .err()
        .unwrap();
        assert_eq!(
            error.code,
            if replace_lock {
                "VERSION_MAINTENANCE_CONFLICT"
            } else {
                "SYNTHETIC_PREFLIGHT"
            }
        );
        assert_eq!(store.read().unwrap().instances[0], source);
        for name in ["version-maintenance", "macos-stops", "run-transitions"] {
            assert!(!store.root().join(name).exists());
        }
    }
}

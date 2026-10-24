use super::*;
struct NoSecrets;
impl Credentials for NoSecrets {
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
#[test]
fn explicit_trial_does_not_override_corrupt_existing_compatibility_material() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let directory = store.root().join("macos-compatibility");
    crate::storage::state::private_dir(&directory).unwrap();
    let foreign = directory.join("foreign.json");
    std::fs::write(&foreign, b"preserve unknown evidence").unwrap();
    let refs = References::with_key(store.root().join("references"), [87; 32]);
    let context = Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    for trial in [false, true] {
        let result = execute_with(
            &context,
            Options {
                instance: None,
                version: None,
                request: Some("original"),
                new: true,
                verify_compatibility: trial,
            },
            None,
            Instant::now() + std::time::Duration::from_secs(5),
            |_, _, _, _| panic!("corrupt evidence must stop before preparation"),
        );
        assert_eq!(result.err().unwrap().code, "STATE_INVALID");
    }
    assert_eq!(
        std::fs::read(&foreign).unwrap(),
        b"preserve unknown evidence"
    );
    assert!(!store.root().join("references").exists());
    assert!(!store.root().join("start-requests").exists());
}
#[test]
fn composed_preparation_can_acquire_its_lifecycle_lock_and_errors_keep_the_start_key() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [60; 32]);
    let context = Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    let deadline = Instant::now() + std::time::Duration::from_secs(1);
    let result = execute_with(
        &context,
        Options {
            instance: None,
            version: None,
            request: Some("original"),
            new: true,
            verify_compatibility: true,
        },
        None,
        deadline,
        |context, request, _, deadline| {
            assert_eq!(request, "original");
            let lock = crate::napcat::runtime::lifecycle_lock(context.store, deadline)?;
            lock.verify()?;
            Err(AppError::new(
                "SYNTHETIC_PREPARATION",
                "Synthetic preparation stopped before any effect",
                4,
            ))
        },
    );
    let error = result.err().unwrap();
    assert_eq!(error.code, "SYNTHETIC_PREPARATION");
    assert_eq!(error.request_id.as_deref(), Some("original"));
    assert!(!store.root().join("start-requests").exists());
    assert!(store.read().unwrap().instances.is_empty());
}

#[test]
fn selected_maintenance_checks_full_snapshots_and_explicit_trial_before_preparation() {
    for version in ["4.18.41", "4.18.38"] {
        selected_version(version);
    }
}
fn selected_version(version: &str) {
    use crate::napcat::runtime::instance_creation::{self, Admission, Creation};
    use crate::storage::state::Installation;
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [61; 32]);
    let release = crate::napcat::install::catalog::lookup(version).unwrap();
    let installed = Installation {
        id: "synthetic-installation".into(),
        version: release.version.into(),
        digest: release.archive.sha256.into(),
        directory: store.root().join("framework"),
    };
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    let mut tx = store.transaction(deadline).unwrap();
    tx.registry.installations.push(installed.clone());
    tx.commit().unwrap();
    // Authenticated protocol metadata only. The preparation callback stops
    // before i1/source verification and cannot produce a real launch.
    let creation = instance_creation::reserve_in(
        store.transaction(deadline).unwrap(),
        Admission {
            refs: &refs,
            request_id: "original-maintenance",
            release: &release,
            installation: installed.clone(),
            kind: "n2",
            guard: &|| Ok(()),
        },
        deadline,
    )
    .unwrap();
    assert!(matches!(creation, Creation::Fresh(_)));
    drop(creation);
    let selected = store.read().unwrap().instances.remove(0);
    let context = Context {
        store: &store,
        references: &refs,
        credentials: &NoSecrets,
    };
    let options = |trial| Options {
        instance: Some(&selected.id),
        version: Some(release.version),
        request: None,
        new: false,
        verify_compatibility: trial,
    };
    let mut automatic = options(false);
    automatic.version = None;
    assert_eq!(
        selection::release(&store.read().unwrap(), &automatic)
            .unwrap()
            .version,
        release.version
    );
    let mut different = options(true);
    different.version = Some(if version == "4.18.38" {
        "4.18.41"
    } else {
        "4.18.38"
    });
    let error = execute_with(
        &context,
        different,
        Some((&selected, &installed)),
        deadline,
        |_, _, _, _| panic!("start cannot silently change the installation"),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "INSTANCE_VERSION_CONFLICT");
    for changed_installation in [false, true] {
        let mut old_instance = selected.clone();
        let mut old_installation = installed.clone();
        if changed_installation {
            old_installation.digest = "b".repeat(64);
        } else {
            old_instance.run_id = "older-run".into();
        }
        let error = execute_with(
            &context,
            options(true),
            Some((&old_instance, &old_installation)),
            deadline,
            |_, _, _, _| panic!("changed snapshot must not prepare"),
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "MACOS_START_CONFLICT");
    }
    let error = execute_with(
        &context,
        options(false),
        Some((&selected, &installed)),
        deadline,
        |_, _, _, _| panic!("unknown compatibility must not prepare"),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "COMPATIBILITY_UNKNOWN");
    let error = execute_with(
        &context,
        options(true),
        Some((&selected, &installed)),
        deadline,
        |context, request, _, deadline| {
            assert_eq!(request, "original-maintenance");
            let lock = crate::napcat::runtime::lifecycle_lock(context.store, deadline)?;
            lock.verify()?;
            Err(AppError::new(
                "SYNTHETIC_MAINTENANCE",
                "No actual preparation",
                4,
            ))
        },
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "SYNTHETIC_MAINTENANCE");
    assert_eq!(store.read().unwrap().instances[0], selected);
    assert!(!store.root().join("start-requests").exists());
    assert!(!store.root().join("run-transitions").exists());
}

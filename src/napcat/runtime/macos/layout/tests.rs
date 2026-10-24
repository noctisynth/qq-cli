use super::*;
use std::time::Duration;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn setup() -> (tempfile::TempDir, Store, References) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let store = Store::new(base.join("state"));
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.installations.push(Installation {
        id: "install".into(),
        version: "synthetic".into(),
        digest: "a".repeat(64),
        directory: base.join("framework"),
    });
    transaction.registry.instances.push(Instance {
        id: "instance".into(),
        installation_id: "install".into(),
        endpoint: "http://127.0.0.1:24001".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: None,
        run_id: "run".into(),
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    transaction.commit().unwrap();
    let references = References::with_key(store.root().join("references"), [17; 32]);
    (temp, store, references)
}

#[test]
fn completion_recovery_scope_and_original_signer() {
    let (_temp, store, refs) = setup();
    let before = store.transaction(deadline()).unwrap().registry.instances[0].clone();
    let Preparation::Ready(first) =
        prepare(&store, &refs, "instance", "request", deadline()).unwrap()
    else {
        panic!("not ready")
    };
    let Preparation::Ready(second) = prepare_with(
        &store,
        &refs,
        "instance",
        "request",
        deadline(),
        |_, _, _, _| panic!("replayed"),
    )
    .unwrap() else {
        panic!("not ready")
    };
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(
        store.transaction(deadline()).unwrap().registry.instances[0],
        before
    );
    assert!(!first.directory().join("adapter").exists());
    assert!(!first.directory().join("QQ.app").exists());
    assert!(
        std::fs::read_dir(first.directory().join("config"))
            .unwrap()
            .next()
            .is_none()
    );
    assert!(prepare(&store, &refs, "instance", "different", deadline()).is_err());
    let wrong = References::with_key(store.root().join("references"), [99; 32]);
    assert!(recover(&store, &wrong, "instance", deadline()).is_err());
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations[0].digest = "b".repeat(64);
    tx.commit().unwrap();
    assert!(recover(&store, &refs, "instance", deadline()).is_err());
}

#[test]
fn failure_and_panic_preserve_partial_material_without_replay() {
    for panic in [false, true] {
        let (_temp, store, refs) = setup();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prepare_with(
                &store,
                &refs,
                "instance",
                "request",
                deadline(),
                |root, _, _, guard| {
                    guard()?;
                    private_dir(root)?;
                    std::fs::write(root.join("partial"), b"preserve").unwrap();
                    if panic {
                        panic!("synthetic interruption")
                    }
                    Err(invalid())
                },
            )
        }));
        if panic {
            assert!(result.is_err());
        } else {
            assert!(matches!(result.unwrap().unwrap(), Preparation::Unknown));
        }
        assert!(matches!(
            recover(&store, &refs, "instance", deadline()).unwrap(),
            Some(Preparation::Unknown)
        ));
        assert!(matches!(
            prepare_with(
                &store,
                &refs,
                "instance",
                "request",
                deadline(),
                |_, _, _, _| panic!("replayed")
            )
            .unwrap(),
            Preparation::Unknown
        ));
        assert!(prepare(&store, &refs, "instance", "different", deadline()).is_err());
        let tx = store.transaction(deadline()).unwrap();
        let (target, _, _) = scope(&tx, "instance").unwrap();
        let (_, root) = paths(&store, &target).unwrap();
        assert_eq!(std::fs::read(root.join("partial")).unwrap(), b"preserve");
    }
}

#[test]
fn changed_registry_or_lock_cannot_publish_completion() {
    for replace in [false, true] {
        let (_temp, store, refs) = setup();
        let result = prepare_with(
            &store,
            &refs,
            "instance",
            "request",
            deadline(),
            |_, _, _, guard| {
                if replace {
                    std::fs::rename(
                        store.root().join("state.lock"),
                        store.root().join("old.lock"),
                    )
                    .unwrap();
                    std::fs::write(store.root().join("state.lock"), b"").unwrap();
                } else {
                    let mut tx = store.transaction(deadline()).unwrap();
                    tx.registry.instances[0].state = "starting".into();
                    tx.commit().unwrap();
                }
                assert!(guard().is_err());
                Err(invalid())
            },
        );
        assert_eq!(result.err().unwrap().code, "MACOS_LAYOUT_OUTCOME_UNKNOWN");
        assert!(matches!(
            recover(&store, &refs, "instance", deadline()).unwrap(),
            Some(Preparation::Unknown)
        ));
    }
}

#[test]
fn preexisting_unclaimed_root_is_preserved() {
    let (_temp, store, refs) = setup();
    let tx = store.transaction(deadline()).unwrap();
    let (target, _, _) = scope(&tx, "instance").unwrap();
    let (index, root) = paths(&store, &target).unwrap();
    drop(tx);
    private_dir(&root).unwrap();
    std::fs::write(root.join("foreign"), b"untouched").unwrap();
    assert!(prepare(&store, &refs, "instance", "request", deadline()).is_err());
    assert!(!index.exists());
    assert_eq!(std::fs::read(root.join("foreign")).unwrap(), b"untouched");
}

fn child(store: &Store, mode: &str) -> std::process::Child {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "napcat::runtime::macos::layout::tests::layout_child_fixture",
            "--nocapture",
        ])
        .env("QQ_CLI_LAYOUT_CHILD", mode)
        .env("QQ_CLI_LAYOUT_STORE", store.root())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap()
}
#[test]
fn layout_child_fixture() {
    use std::io::Write;
    let Ok(mode) = std::env::var("QQ_CLI_LAYOUT_CHILD") else {
        return;
    };
    let store = Store::new(std::env::var_os("QQ_CLI_LAYOUT_STORE").unwrap().into());
    let refs = References::with_key(store.root().join("references"), [17; 32]);
    let result = prepare_with(
        &store,
        &refs,
        "instance",
        if mode == "different" {
            "other"
        } else {
            "request"
        },
        deadline(),
        |root, _, _, guard| {
            guard()?;
            private_dir(root)?;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(root.join("effects"))
                .unwrap();
            file.write_all(b"effect\n").unwrap();
            file.sync_all().unwrap();
            if mode == "crash" {
                std::process::exit(73);
            }
            Err(invalid())
        },
    );
    if mode == "different" {
        assert!(result.is_err());
    } else {
        assert!(matches!(result.unwrap(), Preparation::Unknown));
    }
}
#[test]
fn independent_process_claim_and_abrupt_exit_do_not_replay() {
    for crash in [false, true] {
        let (_temp, store, refs) = setup();
        if crash {
            assert_eq!(child(&store, "crash").wait().unwrap().code(), Some(73));
        } else {
            let mut first = child(&store, "normal");
            let mut second = child(&store, "normal");
            assert!(first.wait().unwrap().success());
            assert!(second.wait().unwrap().success());
        }
        assert!(matches!(
            recover(&store, &refs, "instance", deadline()).unwrap(),
            Some(Preparation::Unknown)
        ));
        assert!(child(&store, "normal").wait().unwrap().success());
        assert!(child(&store, "different").wait().unwrap().success());
        let tx = store.transaction(deadline()).unwrap();
        let (target, _, _) = scope(&tx, "instance").unwrap();
        let (_, root) = paths(&store, &target).unwrap();
        assert_eq!(std::fs::read(root.join("effects")).unwrap(), b"effect\n");
    }
}

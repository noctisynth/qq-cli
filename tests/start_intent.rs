use qq_cli::napcat::runtime::start_intent::{self, Begin, Outcome, Target};
use qq_cli::storage::references::References;
use qq_cli::storage::state::{Installation, Instance, Store};
use std::time::{Duration, Instant};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn original_lock_is_required_before_dispatch_and_after_effects() {
    for after_effect in [false, true] {
        for replace_directory in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let store = Store::new(temp.path().join("state"));
            let refs = refs(&store);
            setup(&store);
            let Begin::Fresh(intent) =
                start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
            else {
                panic!("fresh intent expected")
            };
            let original = if replace_directory {
                store.root().to_owned()
            } else {
                store.root().join("state.lock")
            };
            let saved = original.with_extension("saved");
            let replace = || {
                std::fs::rename(&original, &saved).unwrap();
                if replace_directory {
                    std::fs::create_dir(&original).unwrap();
                } else {
                    std::fs::write(&original, b"replacement").unwrap();
                }
            };
            if !after_effect {
                replace();
            }
            let mut calls = 0;
            let error = intent
                .dispatch(|_| {
                    calls += 1;
                    replace();
                    Ok(())
                })
                .unwrap_err();
            assert_eq!(calls, usize::from(after_effect));
            assert_eq!(
                error.code,
                if after_effect {
                    "START_OUTCOME_UNKNOWN"
                } else {
                    "STATE_IO"
                }
            );
            assert!(!error.retry_safe);
            if after_effect {
                assert_eq!(
                    error.outcome,
                    Some(qq_cli::core::contracts::write::Outcome::Unknown)
                );
                assert_eq!(error.request_id.as_deref(), Some("key"));
            }
            if replace_directory {
                assert_eq!(std::fs::read_dir(&original).unwrap().count(), 0);
                std::fs::remove_dir(&original).unwrap();
            } else {
                std::fs::remove_file(&original).unwrap();
            }
            std::fs::rename(&saved, &original).unwrap();
            assert_eq!(
                start_intent::get(&store, &refs, "key").unwrap().outcome,
                Outcome::Pending
            );
            let Begin::Existing(record) =
                start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
            else {
                panic!("original run must not dispatch again")
            };
            assert_eq!(record.outcome, Outcome::Unknown);
        }
    }
}
fn target() -> Target {
    Target {
        instance_id: "synthetic-instance".into(),
        run_id: "synthetic-run".into(),
        installation_id: "synthetic-installation".into(),
        installation_digest: "a".repeat(64),
        input_hash: "b".repeat(64),
    }
}
fn setup(store: &Store) {
    let t = target();
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.installations.push(Installation {
        id: t.installation_id.clone(),
        version: "v0.0.0".into(),
        digest: t.installation_digest,
        directory: store.root().join("unused-original-fixture"),
    });
    tx.registry.instances.push(Instance {
        id: t.instance_id,
        installation_id: t.installation_id,
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "unused-synthetic".into(),
        account: None,
        run_id: t.run_id,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
}
fn refs(store: &Store) -> References {
    References::with_key(store.root().join("references"), [31; 32])
}
#[test]
fn dispatch_is_durable_and_never_claims_process_or_identity_proof() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = refs(&store);
    assert_eq!(
        start_intent::get(&store, &refs, "key").unwrap_err().code,
        "REQUEST_NOT_FOUND"
    );
    assert!(!store.root().exists());
    setup(&store);
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("fresh intent expected")
    };
    let record = intent
        .dispatch(|tx| {
            assert_eq!(tx.registry.instances[0].state, "starting");
            assert_eq!(
                start_intent::get(&store, &refs, "key")?.outcome,
                Outcome::Pending
            );
            Ok(())
        })
        .unwrap();
    assert_eq!(record.outcome, Outcome::Dispatched);
    let instance = &store.read().unwrap().instances[0];
    assert_eq!(instance.state, "starting");
    assert!(
        instance.process_id.is_none()
            && instance.process_started.is_none()
            && instance.account.is_none()
    );
    let Begin::Existing(record) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("must not replay")
    };
    assert_eq!(record.outcome, Outcome::Dispatched);
    let mut changed = target();
    changed.input_hash = "c".repeat(64);
    assert_eq!(
        start_intent::begin(&store, &refs, "key", &changed, deadline())
            .err()
            .unwrap()
            .code,
        "START_CONFLICT"
    );
    assert_eq!(
        start_intent::begin(&store, &refs, "different-key", &target(), deadline())
            .err()
            .unwrap()
            .code,
        "START_CONFLICT"
    );
}
#[test]
fn interruption_and_uncertain_dispatch_are_not_replayed() {
    for failure in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = refs(&store);
        setup(&store);
        let Begin::Fresh(intent) =
            start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
        else {
            panic!("fresh intent expected")
        };
        if failure {
            assert_eq!(
                intent
                    .dispatch(|_| Err(qq_cli::core::error::AppError::storage()))
                    .unwrap()
                    .outcome,
                Outcome::Unknown
            );
        } else {
            drop(intent);
        }
        let Begin::Existing(record) =
            start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
        else {
            panic!("must not replay")
        };
        assert_eq!(record.outcome, Outcome::Unknown);
        assert_eq!(
            start_intent::get(&store, &refs, "key").unwrap().outcome,
            Outcome::Unknown
        );
    }
}

#[test]
fn run_claim_survives_crash_before_request_index_and_starting_commit() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = refs(&store);
    setup(&store);
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("fresh intent expected")
    };
    drop(intent);
    // Original synthetic layout of a crash after immutable run claim, but before
    // publishing the request index or marking the registry starting.
    let journal = std::fs::read_dir(store.root().join("start-requests"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(journal.starts_with(temp.path()));
    std::fs::remove_file(&journal).unwrap();
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "prepared".into();
    tx.commit().unwrap();
    assert_eq!(
        start_intent::begin(&store, &refs, "different-key", &target(), deadline())
            .err()
            .unwrap()
            .code,
        "START_CONFLICT"
    );
    let Begin::Existing(record) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("a claimed run must never be dispatched again")
    };
    assert_eq!(record.outcome, Outcome::Unknown);
    assert_eq!(
        start_intent::get(&store, &refs, "key").unwrap().outcome,
        Outcome::Unknown
    );
    assert_eq!(store.read().unwrap().instances[0].state, "prepared");
}
#[test]
fn invalid_phase_expiry_and_corrupt_journals_cannot_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = refs(&store);
    assert_eq!(
        start_intent::begin(&store, &refs, "key", &target(), Instant::now())
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    assert!(!store.root().exists());
    setup(&store);
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "ready".into();
    tx.commit().unwrap();
    assert_eq!(
        start_intent::begin(&store, &refs, "key", &target(), deadline())
            .err()
            .unwrap()
            .code,
        "START_CONFLICT"
    );
    assert!(!store.root().join("start-requests").exists());
    let mut tx = store.transaction(deadline()).unwrap();
    tx.registry.instances[0].state = "prepared".into();
    tx.commit().unwrap();
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("fresh intent expected")
    };
    drop(intent);
    let journal = std::fs::read_dir(store.root().join("start-requests"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(&journal, b"{}").unwrap();
    assert!(start_intent::begin(&store, &refs, "key", &target(), deadline()).is_err());
    assert_eq!(std::fs::read(&journal).unwrap(), b"{}");
    assert_eq!(store.read().unwrap().instances[0].state, "starting");
}
#[test]
fn competing_callers_share_one_dispatch_opportunity() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    setup(&store);
    let count = std::sync::atomic::AtomicUsize::new(0);
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                let refs = refs(&store);
                barrier.wait();
                match start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap() {
                    Begin::Fresh(intent) => {
                        intent
                            .dispatch(|_| {
                                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                Ok(())
                            })
                            .unwrap();
                    }
                    Begin::Existing(record) => assert_eq!(record.outcome, Outcome::Dispatched),
                }
            });
        }
    });
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn receipt_commit_failure_preserves_unknown_without_a_second_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = refs(&store);
    setup(&store);
    let Begin::Fresh(intent) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("fresh intent expected")
    };
    assert_eq!(
        start_intent::begin(
            &store,
            &refs,
            "key",
            &target(),
            Instant::now() + Duration::from_millis(30)
        )
        .err()
        .unwrap()
        .code,
        "STATE_BUSY"
    );
    let source = store.root().join("start-requests");
    let saved = store.root().join("saved-start-requests");
    assert!(source.starts_with(temp.path()) && saved.starts_with(temp.path()) && !saved.exists());
    let error = intent
        .dispatch(|_| {
            std::fs::rename(&source, &saved).unwrap();
            std::fs::write(&source, b"original synthetic obstruction").unwrap();
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "START_OUTCOME_UNKNOWN");
    assert_eq!(
        error.outcome,
        Some(qq_cli::core::contracts::write::Outcome::Unknown)
    );
    assert_eq!(error.request_id.as_deref(), Some("key"));
    assert!(!error.retry_safe);
    assert_eq!(
        std::fs::read(&source).unwrap(),
        b"original synthetic obstruction"
    );
    std::fs::remove_file(&source).unwrap();
    std::fs::rename(&saved, &source).unwrap();
    let Begin::Existing(record) =
        start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap()
    else {
        panic!("must not dispatch again")
    };
    assert_eq!(record.outcome, Outcome::Unknown);
}

#[test]
fn independent_processes_and_abrupt_exit_never_repeat_dispatch() {
    use std::process::{Command, Stdio};
    const ROOT: &str = "QQ_CLI_TEST_START_ROOT";
    const CRASH: &str = "QQ_CLI_TEST_START_CRASH";
    if let Some(root) = std::env::var_os(ROOT) {
        let root = std::path::PathBuf::from(root);
        let store = Store::new(root.join("state"));
        let refs = refs(&store);
        match start_intent::begin(&store, &refs, "key", &target(), deadline()).unwrap() {
            Begin::Fresh(intent) => {
                intent
                    .dispatch(|_| {
                        use std::io::Write;
                        let mut marker = std::fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(root.join("dispatch-once.txt"))
                            .unwrap();
                        marker
                            .write_all(b"one original synthetic dispatch")
                            .unwrap();
                        marker.sync_all().unwrap();
                        if std::env::var_os(CRASH).is_some() {
                            std::process::exit(23);
                        }
                        Ok(())
                    })
                    .unwrap();
            }
            Begin::Existing(record) => {
                assert!(matches!(
                    record.outcome,
                    Outcome::Dispatched | Outcome::Unknown
                ));
            }
        }
        return;
    }
    let launch = |root: &std::path::Path, crash: bool| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "independent_processes_and_abrupt_exit_never_repeat_dispatch",
                "--test-threads=1",
            ])
            .env(ROOT, root)
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if crash {
            command.env(CRASH, "1");
        } else {
            command.env_remove(CRASH);
        }
        command.spawn().unwrap()
    };
    let wait = |mut child: std::process::Child| {
        let until = deadline();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            if Instant::now() >= until {
                // Only this test's own returned child handle is eligible for cleanup.
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("synthetic child timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    for crash in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        setup(&store);
        let first = launch(temp.path(), crash);
        if crash {
            assert_eq!(wait(first).code(), Some(23));
            assert!(wait(launch(temp.path(), false)).success());
        } else {
            let second = launch(temp.path(), false);
            assert!(wait(first).success());
            assert!(wait(second).success());
        }
        assert_eq!(
            std::fs::read(temp.path().join("dispatch-once.txt")).unwrap(),
            b"one original synthetic dispatch"
        );
        assert_eq!(
            start_intent::get(&store, &refs(&store), "key")
                .unwrap()
                .outcome,
            if crash {
                Outcome::Unknown
            } else {
                Outcome::Dispatched
            }
        );
    }
}

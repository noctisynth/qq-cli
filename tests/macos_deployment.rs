#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::{
    core::error::AppError,
    napcat::{
        install::macos::{
            bundle::{BundleRole, Inventory},
            signing::Policy,
        },
        runtime::macos::deploy::{self, Status, Target},
    },
    storage::{
        references::References,
        state::{Installation, Instance, Store},
    },
};
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn setup() -> (tempfile::TempDir, Store, References, Target) {
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
    let target = Target {
        instance_id: "instance".into(),
        run_id: "run".into(),
        installation_id: "install".into(),
        installation_digest: "a".repeat(64),
        input_hash: "b".repeat(64),
    };
    (temp, store, references, target)
}
fn root(store: &Store, target: &Target) -> PathBuf {
    let id = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(&target.instance_id, &target.run_id)).unwrap())
    );
    store.root().join("runtimes").join(id).join("QQ.app")
}
#[test]
fn failure_or_interruption_never_replays_even_with_a_different_key() {
    for panic in [false, true] {
        let (_temp, store, refs, target) = setup();
        let calls = Cell::new(0);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            deploy::deploy(&store, &refs, "request", &target, deadline(), |_, guard| {
                guard()?;
                calls.set(calls.get() + 1);
                if panic {
                    panic!("synthetic interruption");
                }
                Err(AppError::new("SYNTHETIC_FAILURE", "Synthetic failure", 3))
            })
        }));
        if !panic {
            assert_eq!(result.unwrap().unwrap(), Status::Unknown);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(
            deploy::recover(&store, &refs, &target, deadline()).unwrap(),
            Some(Status::Unknown)
        );
        assert_eq!(
            deploy::deploy(
                &store,
                &refs,
                "request",
                &target,
                deadline(),
                |_, _| panic!("replayed")
            )
            .unwrap(),
            Status::Unknown
        );
        assert!(
            deploy::deploy(
                &store,
                &refs,
                "different",
                &target,
                deadline(),
                |_, _| panic!("replayed")
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
}
#[test]
fn an_inspected_tree_cannot_be_published_as_a_completed_deployment() {
    let (_temp, store, refs, target) = setup();
    assert_eq!(
        deploy::deploy(
            &store,
            &refs,
            "request",
            &target,
            deadline(),
            |root, guard| {
                guard()?;
                std::fs::create_dir_all(root).unwrap();
                std::fs::write(root.join("data"), b"unsigned").unwrap();
                Inventory::inspect(root, deadline())
            }
        )
        .unwrap(),
        Status::Unknown
    );
}
#[test]
fn changed_registry_or_replaced_state_lock_preserves_unknown_outcome() {
    for replace in [false, true] {
        let (_temp, store, refs, target) = setup();
        let result = deploy::deploy(&store, &refs, "request", &target, deadline(), |_, guard| {
            guard()?;
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
            Err(AppError::new("SYNTHETIC_FAILURE", "Synthetic failure", 3))
        });
        assert_eq!(result.unwrap_err().code, "DEPLOYMENT_OUTCOME_UNKNOWN");
        assert_eq!(
            deploy::recover(&store, &refs, &target, deadline()).unwrap(),
            Some(Status::Unknown)
        );
    }
}
fn source(base: &Path) -> PathBuf {
    let app = base.join("Synthetic.app");
    std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    std::fs::create_dir_all(app.join("Contents/Resources")).unwrap();
    std::fs::copy(
        std::env::current_exe().unwrap(),
        app.join("Contents/MacOS/Main"),
    )
    .unwrap();
    std::fs::write(app.join("Contents/Info.plist"),b"<?xml version=\"1.0\"?><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>org.qq-cli.synthetic</string><key>CFBundleExecutable</key><string>Main</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>").unwrap();
    std::fs::write(app.join("Contents/Resources/data"), b"synthetic resource").unwrap();
    assert!(
        std::process::Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .arg(&app)
            .output()
            .unwrap()
            .status
            .success()
    );
    app
}
#[test]
fn completed_signed_copy_is_authenticated_rechecked_and_never_reexecuted() {
    let (temp, store, refs, target) = setup();
    let source = source(&temp.path().canonicalize().unwrap());
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let result = deploy::deploy(
        &store,
        &refs,
        "request",
        &target,
        deadline(),
        |root, guard| {
            guard()?;
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(root.parent().unwrap())
                .unwrap();
            let copy = original
                .copy_to_new(root.parent().unwrap(), "QQ.app", deadline(), guard)?
                .harden_copy(deadline(), guard)?;
            let plan = copy.signing_plan(
                &[BundleRole::new(
                    Path::new(""),
                    "org.qq-cli.synthetic",
                    "Main",
                    Policy::Main,
                )?],
                deadline(),
            )?;
            copy.sign_copy(&plan, deadline(), guard)
        },
    )
    .unwrap();
    assert_eq!(result, Status::Complete);
    assert_eq!(
        deploy::recover(&store, &refs, &target, deadline()).unwrap(),
        Some(Status::Complete)
    );
    assert_eq!(
        deploy::deploy(
            &store,
            &refs,
            "request",
            &target,
            deadline(),
            |_, _| panic!("replayed")
        )
        .unwrap(),
        Status::Complete
    );
    let mut changed = target.clone();
    changed.input_hash = "c".repeat(64);
    assert!(deploy::recover(&store, &refs, &changed, deadline()).is_err());
    let wrong = References::with_key(store.root().join("references"), [99; 32]);
    assert!(deploy::recover(&store, &wrong, &target, deadline()).is_err());
    std::fs::write(
        root(&store, &target).join("Contents/Resources/data"),
        b"changed",
    )
    .unwrap();
    assert!(deploy::recover(&store, &refs, &target, deadline()).is_err());
    original.revalidate(deadline()).unwrap();
}

fn child(store: &Store, mode: &str) -> std::process::Child {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "deployment_child_fixture", "--nocapture"])
        .env("QQ_CLI_DEPLOY_CHILD", mode)
        .env("QQ_CLI_DEPLOY_STORE", store.root())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap()
}
#[test]
fn deployment_child_fixture() {
    use std::io::Write;
    let Ok(mode) = std::env::var("QQ_CLI_DEPLOY_CHILD") else {
        return;
    };
    let store = Store::new(std::env::var_os("QQ_CLI_DEPLOY_STORE").unwrap().into());
    let refs = References::with_key(store.root().join("references"), [17; 32]);
    let target = Target {
        instance_id: "instance".into(),
        run_id: "run".into(),
        installation_id: "install".into(),
        installation_digest: "a".repeat(64),
        input_hash: "b".repeat(64),
    };
    let result = deploy::deploy(
        &store,
        &refs,
        if mode == "different" {
            "other"
        } else {
            "request"
        },
        &target,
        deadline(),
        |_, guard| {
            guard()?;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(store.root().join("effects"))
                .unwrap();
            file.write_all(b"effect\n").unwrap();
            file.sync_all().unwrap();
            if mode == "crash" {
                std::process::exit(73);
            }
            Err(AppError::new("SYNTHETIC_FAILURE", "Synthetic failure", 3))
        },
    );
    if mode == "different" {
        assert!(result.is_err());
    } else {
        assert_eq!(result.unwrap(), Status::Unknown);
    }
}
#[test]
fn independent_processes_share_one_deployment_claim() {
    let (_temp, store, _refs, _target) = setup();
    let mut first = child(&store, "normal");
    let mut second = child(&store, "normal");
    assert!(first.wait().unwrap().success());
    assert!(second.wait().unwrap().success());
    assert_eq!(
        std::fs::read(store.root().join("effects")).unwrap(),
        b"effect\n"
    );
}
#[test]
fn abrupt_process_exit_retains_claim_and_blocks_replay_across_keys() {
    let (_temp, store, refs, target) = setup();
    assert_eq!(child(&store, "crash").wait().unwrap().code(), Some(73));
    assert_eq!(
        deploy::recover(&store, &refs, &target, deadline()).unwrap(),
        Some(Status::Unknown)
    );
    assert!(child(&store, "normal").wait().unwrap().success());
    assert!(child(&store, "different").wait().unwrap().success());
    assert_eq!(
        std::fs::read(store.root().join("effects")).unwrap(),
        b"effect\n"
    );
}

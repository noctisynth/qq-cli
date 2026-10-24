//! Standalone verification host, compiled against the current library. Only
//! synthetic mode copies/signs an application; source mode is strictly read-only
//! for QQ and imports the pinned Framework into a fresh synthetic state directory.
use qq_cli::{
    core::error::AppError,
    napcat::{
        install::{
            catalog, installation,
            macos_bundle::{self, BundleRole, Inventory},
            macos_signing::{self, Policy},
            macos_source::Source,
        },
        runtime::{
            instance_creation::Creation,
            macos_adapter::Prepared,
            macos_assembly::{Assembly, Plan},
            macos_creation,
            macos_deploy::{self, Status, Target},
            macos_layout::{self, Preparation},
            macos_material,
        },
    },
    native::{macos_copy_worker, macos_signature},
    storage::{
        credentials::Credentials,
        references::References,
        state::{Installation, Instance, Store},
    },
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    fs::File,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const RUN: &str = "510723a3a7309d967791de533b255b09c454b7ae9bf3a2cea6c7007c3a888d53";
const PACKAGE: &[u8] = br#"{"name":"qq-chat","platform":"darwin","version":"fixture","buildVersion":"1","main":"./application.asar/app_launcher/index.js"}"#;
const PACKAGE_DIGEST: &str = "c79bd697f0c40ad8d9edeae898f23e9c0b8726329ee41f2fa409cddb353b95d6";
fn directory(path: &Path) {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
}
fn instance(installation_id: &str) -> Instance {
    Instance {
        id: "instance".into(),
        installation_id: installation_id.into(),
        endpoint: "http://127.0.0.1:24001".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: None,
        run_id: "run".into(),
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    }
}
fn layout(store: &Store, refs: &References, deadline: Instant) -> macos_layout::Layout {
    let Preparation::Ready(layout) =
        macos_layout::prepare(store, refs, "instance", "layout", deadline).unwrap()
    else {
        panic!("layout preparation unresolved");
    };
    layout
}
fn source(base: &Path) -> PathBuf {
    let root = base.join("Synthetic.app");
    directory(&root.join("Contents/MacOS"));
    directory(&root.join("Contents/Resources/app"));
    std::fs::copy(
        std::env::current_exe().unwrap(),
        root.join("Contents/MacOS/Main"),
    )
    .unwrap();
    std::fs::write(root.join("Contents/Resources/app/package.json"), PACKAGE).unwrap();
    std::fs::write(
        root.join("Contents/Resources/data"),
        b"original synthetic resource",
    )
    .unwrap();
    std::fs::write(root.join("Contents/Info.plist"), br#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.qq-cli.synthetic</string><key>CFBundleExecutable</key><string>Main</string><key>CFBundlePackageType</key><string>APPL</string></dict></plist>"#).unwrap();
    assert!(
        std::process::Command::new("/usr/bin/codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .arg(&root)
            .output()
            .unwrap()
            .status
            .success()
    );
    root
}
fn synthetic(base: &Path) {
    let deadline = Instant::now() + Duration::from_secs(180);
    let source = Inventory::inspect(&source(base), deadline).unwrap();
    let roles =
        [BundleRole::new(Path::new(""), "org.qq-cli.synthetic", "Main", Policy::Main).unwrap()];
    for fail in [false, true] {
        let store = Store::new(base.join(if fail { "failed" } else { "completed" }));
        let refs = References::with_key(store.root().join("references"), [17; 32]);
        let framework = base.join("framework");
        let installation = Installation {
            id: "install".into(),
            version: "synthetic".into(),
            digest: "a".repeat(64),
            directory: framework.clone(),
        };
        let mut tx = store.transaction(deadline).unwrap();
        tx.registry.instances.push(instance("install"));
        tx.registry.installations.push(installation);
        tx.commit().unwrap();
        let root = store.root().join("runtimes").join(RUN);
        directory(&framework);
        let layout = layout(&store, &refs, deadline);
        assert_eq!(layout.directory(), root);
        let prepared = Prepared::inspect(&root, &framework).unwrap();
        let adapter = prepared.adapter_plan().unwrap();
        let package = prepared
            .plan_package_edit(PACKAGE, PACKAGE_DIGEST, "fixture", "1")
            .unwrap();
        let target = Target {
            instance_id: "instance".into(),
            run_id: "run".into(),
            installation_id: "install".into(),
            installation_digest: "a".repeat(64),
            input_hash: "b".repeat(64),
        };
        let calls = Cell::new(0);
        let status = macos_deploy::deploy(
            &store,
            &refs,
            "request",
            &target,
            deadline,
            |root, guard| {
                Assembly {
                    source: &source,
                    roles: &roles,
                    prepared: &prepared,
                    adapter: &adapter,
                    package: &package,
                }
                .execute(root, deadline, || {
                    guard()?;
                    layout.verify(deadline)?;
                    calls.set(calls.get() + 1);
                    if fail && calls.get() >= 6 {
                        Err(AppError::new(
                            "SYNTHETIC_FAILURE",
                            "Synthetic failure after publication",
                            3,
                        ))
                    } else {
                        Ok(())
                    }
                })
            },
        )
        .unwrap();
        let expected = if fail {
            Status::Unknown
        } else {
            Status::Complete
        };
        assert_eq!(status, expected);
        assert_eq!(
            macos_deploy::recover(&store, &refs, &target, deadline).unwrap(),
            Some(expected)
        );
        assert_eq!(
            macos_deploy::deploy(&store, &refs, "request", &target, deadline, |_, _| panic!(
                "replayed"
            ))
            .unwrap(),
            expected
        );
        if fail {
            assert!(adapter.entry().exists());
            assert!(!root.join("QQ.app").exists());
        } else {
            package
                .verify_result(&std::fs::read(package.destination()).unwrap())
                .unwrap();
        }
        source.revalidate(deadline).unwrap();
        println!("synthetic assembly {status:?}, recovery and no-replay verified");
    }
}
#[derive(Default)]
struct Memory(RefCell<BTreeMap<String, String>>);
impl Credentials for Memory {
    fn get(&self, key: &str) -> qq_cli::core::error::Result<String> {
        self.0
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::new("CREDENTIAL_NOT_FOUND", "Synthetic key absent", 4))
    }
    fn set(&self, key: &str, value: &str) -> qq_cli::core::error::Result<()> {
        self.0.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, _: &str) -> qq_cli::core::error::Result<()> {
        panic!("no vault deletion")
    }
}
fn original(base: &Path, app: &Path, archive: &Path) {
    let deadline = Instant::now() + Duration::from_secs(1200);
    let source = Source::inspect(app, deadline).unwrap();
    println!("fixed original source authenticated");
    let store = Store::new(base.join("state"));
    let refs = References::with_key(store.root().join("references"), [19; 32]);
    installation::import(
        File::open(archive).unwrap(),
        &catalog::lookup("4.18.41").unwrap(),
        &store,
        &refs,
        deadline,
    )
    .unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let Creation::Fresh(reservation) =
        macos_creation::reserve(&store, &refs, &source, &installed.id, "creation", deadline)
            .unwrap()
    else {
        panic!("fresh reservation required")
    };
    let runtime = reservation.plan().clone();
    let checkpoint = macos_creation::checkpoint(&store, &refs, "creation")
        .unwrap()
        .unwrap();
    assert!(checkpoint.plan == runtime);
    let Preparation::Ready(layout) =
        macos_layout::prepare(&store, &refs, &runtime.instance_id, "layout", deadline).unwrap()
    else {
        panic!("layout unresolved")
    };
    let root = layout.directory().to_owned();
    let memory = Memory::default();
    let macos_material::Preparation::Ready(material) =
        macos_material::prepare(&store, &refs, &memory, &runtime, "material", deadline).unwrap()
    else {
        panic!("material unresolved")
    };
    assert_eq!(material.directory(), root);
    assert!(matches!(
        macos_material::recover(&store, &refs, &memory, &runtime, deadline).unwrap(),
        Some(macos_material::Preparation::Ready(_))
    ));
    let plan = Plan::inspect(&store, &refs, &source, &runtime.instance_id, deadline).unwrap();
    assert_eq!(plan.target().input_hash.len(), 64);
    assert!(!root.join("QQ.app").exists() && !root.join("adapter").exists());
    assert!(!store.root().join("macos-deployments").exists());
    let mut tx = store.transaction(deadline).unwrap();
    tx.registry.instances[0].endpoint = "http://127.0.0.1:24002".into();
    tx.commit().unwrap();
    assert_eq!(
        plan.revalidate(&store, &refs, deadline).unwrap_err().code,
        "MACOS_ASSEMBLY_CONFLICT"
    );
    println!(
        "authenticated reservation, layout, in-memory credentials, configuration and input planning verified; no QQ deployment/start"
    );
}
fn main() {
    for worker in [
        macos_bundle::supervisor_entry as fn() -> Option<u8>,
        macos_signing::worker_entry,
        macos_copy_worker::worker_entry,
        macos_signature::worker_entry,
    ] {
        if let Some(code) = worker() {
            std::process::exit(code.into());
        }
    }
    let args: Vec<_> = std::env::args().collect();
    let base = Path::new(&args[2]);
    std::fs::DirBuilder::new().mode(0o700).create(base).unwrap();
    assert_eq!(base.canonicalize().unwrap(), base);
    match args[1].as_str() {
        "synthetic" => synthetic(base),
        "source" => original(base, Path::new(&args[3]), Path::new(&args[4])),
        _ => panic!("unknown probe mode"),
    }
    std::fs::remove_dir_all(base).unwrap();
}

//! Standalone verification host, compiled against the current library. Only
//! synthetic mode copies/signs an application; source mode is strictly read-only
//! for QQ and imports the pinned Framework into a fresh synthetic state directory.
use qq_cli::{
    core::error::AppError,
    napcat::{
        install::{
            catalog, installation,
            macos::{
                bundle::{BundleRole, Inventory},
                signing::Policy,
                source::Source,
            },
        },
        runtime::{
            instance_creation::Creation,
            macos::{
                adapter::Prepared,
                assembly::{Assembly, Plan},
                creation,
                deploy::{self, Status, Target},
                layout::{self, Preparation},
                material,
            },
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
fn layout(store: &Store, refs: &References, deadline: Instant) -> layout::Layout {
    let Preparation::Ready(layout) =
        layout::prepare(store, refs, "instance", "layout", deadline).unwrap()
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
        let status = deploy::deploy(
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
            deploy::recover(&store, &refs, &target, deadline).unwrap(),
            Some(expected)
        );
        assert_eq!(
            deploy::deploy(&store, &refs, "request", &target, deadline, |_, _| panic!(
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
        creation::reserve(&store, &refs, &source, &installed.id, "creation", deadline).unwrap()
    else {
        panic!("fresh reservation required")
    };
    let runtime = reservation.plan().clone();
    let checkpoint = creation::checkpoint(&store, &refs, "creation")
        .unwrap()
        .unwrap();
    assert!(checkpoint.plan == runtime);
    let Preparation::Ready(layout) =
        layout::prepare(&store, &refs, &runtime.instance_id, "layout", deadline).unwrap()
    else {
        panic!("layout unresolved")
    };
    let root = layout.directory().to_owned();
    let memory = Memory::default();
    let material::Preparation::Ready(material) =
        material::prepare(&store, &refs, &memory, &runtime, "material", deadline).unwrap()
    else {
        panic!("material unresolved")
    };
    assert_eq!(material.directory(), root);
    assert!(matches!(
        material::recover(&store, &refs, &memory, &runtime, deadline).unwrap(),
        Some(material::Preparation::Ready(_))
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
fn image_source(base: &Path, input: &Path) {
    use qq_cli::napcat::install::macos::source::image::{Image, acquire};
    let deadline = Instant::now() + Duration::from_secs(300);
    let input = input.canonicalize().unwrap();
    let original = Image::inspect(&input, deadline).unwrap();
    let store = Store::new(base.join("state"));
    let cached = acquire::acquire(&store, Some(&input), deadline).unwrap();
    cached.revalidate(deadline).unwrap();
    let reused = acquire::acquire(&store, Some(&base.join("absent-source")), deadline).unwrap();
    assert_eq!(cached.path(), reused.path());
    original.revalidate(deadline).unwrap();
    assert!(!store.root().join("registry.json").exists());
    println!(
        "fixed DMG acquired, cached without reopening source, and original preserved; no mount or QQ execution"
    );
}
fn mounted_source(input: &Path, mount: &Path) {
    use qq_cli::napcat::install::macos::source::image::Image;
    let deadline = Instant::now() + Duration::from_secs(300);
    let image = Image::inspect(&input.canonicalize().unwrap(), deadline).unwrap();
    verify_mounted_source(&image, mount, deadline);
}
fn transition_source(input: &Path, mount: &Path) {
    use qq_cli::napcat::install::macos::source::image::{Image, transition::MountTransition};
    use std::io::{Read, Write};
    let deadline = Instant::now() + Duration::from_secs(300);
    let image = Image::inspect(&input.canonicalize().unwrap(), deadline).unwrap();
    let transition = MountTransition::prepare(image, deadline, &|| Ok(())).unwrap();
    transition.verify_before(deadline, &|| Ok(())).unwrap();
    println!("PREPARED");
    std::io::stdout().flush().unwrap();
    let mut signal = [0; 8];
    std::io::stdin().read_exact(&mut signal).unwrap();
    assert_eq!(&signal, b"mounted\n");
    let image = transition.finish(deadline, &|| Ok(())).unwrap();
    verify_mounted_source(&image, mount, deadline);
    println!("pre-attach snapshot and post-attach metadata transition verified");
}
fn verify_mounted_source(
    image: &qq_cli::napcat::install::macos::source::image::Image,
    mount: &Path,
    deadline: Instant,
) {
    use qq_cli::napcat::install::macos::source::{Source, image::mounted::Observation};
    let observed = Observation::inspect(image, mount, deadline, &|| Ok(())).unwrap();
    let source = Source::inspect(&observed.root().join("QQ.app"), deadline).unwrap();
    source.revalidate(deadline).unwrap();
    observed.revalidate(deadline, &|| Ok(())).unwrap();
    println!(
        "fixed image, existing read-only mount and original QQ Source verified; no mount ownership or QQ execution"
    );
}
fn supervised_mount(base: &Path, input: &Path, release: bool, durable: bool) {
    use qq_cli::napcat::install::macos::source::{
        Source,
        image::{acquire, attach, journal::Status},
    };
    use qq_cli::storage::{references::References, state::Store};
    let deadline = Instant::now() + Duration::from_secs(300);
    let store = Store::new(base.join("state"));
    let references = References::with_key(store.root().join("references"), [74; 32]);
    let image = acquire::acquire(&store, Some(input), deadline).unwrap();
    let Status::Ready(ready) =
        attach::attach(&store, &references, "probe-mount", image, deadline).unwrap()
    else {
        panic!("mount outcome unknown; retain probe scene");
    };
    let source = Source::inspect(&ready.root().join("QQ.app"), deadline).unwrap();
    source.revalidate(deadline).unwrap();
    ready.revalidate(deadline, &|| Ok(())).unwrap();
    let image = acquire::acquire(&store, None, deadline).unwrap();
    let Status::Ready(recovered) =
        attach::attach(&store, &references, "probe-mount", image, deadline).unwrap()
    else {
        panic!("completed mount not recovered");
    };
    recovered.revalidate(deadline, &|| Ok(())).unwrap();
    println!("supervised fixed-image attach, Source and read-only recovery verified");
    if durable {
        use qq_cli::napcat::install::macos::source::image::{
            journal::release::Status as ReleaseStatus, release as releasing,
        };
        drop(source);
        drop(ready);
        let image = (*recovered).into_image();
        let status = releasing::release(
            &store,
            &references,
            "probe-mount",
            "probe-release",
            image,
            deadline,
        )
        .unwrap();
        assert_eq!(status, ReleaseStatus::Released);
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert_eq!(
            releasing::release(
                &store,
                &references,
                "probe-mount",
                "probe-release",
                image,
                deadline,
            )
            .unwrap(),
            ReleaseStatus::Released
        );
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert!(matches!(
            attach::attach(&store, &references, "probe-mount", image, deadline).unwrap(),
            Status::Released
        ));
        use qq_cli::napcat::install::macos::source::image::journal::cleanup;
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert_eq!(
            cleanup::cleanup(&store, &references, "probe-mount", image, deadline).unwrap(),
            cleanup::Status::Complete
        );
        let image = acquire::acquire(&store, None, deadline).unwrap();
        let Status::Ready(next) =
            attach::attach(&store, &references, "probe-next", image, deadline).unwrap()
        else {
            panic!("new mount did not follow cleanup");
        };
        let source = Source::inspect(&next.root().join("QQ.app"), deadline).unwrap();
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert_eq!(
            cleanup::cleanup(&store, &references, "probe-mount", image, deadline).unwrap(),
            cleanup::Status::Complete
        );
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert_eq!(
            releasing::release(
                &store,
                &references,
                "probe-mount",
                "probe-release",
                image,
                deadline
            )
            .unwrap(),
            ReleaseStatus::Released
        );
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert!(matches!(
            attach::attach(&store, &references, "probe-mount", image, deadline).unwrap(),
            Status::Released
        ));
        source.revalidate(deadline).unwrap();
        drop(source);
        let image = (*next).into_image();
        assert_eq!(
            releasing::release(
                &store,
                &references,
                "probe-next",
                "probe-next-release",
                image,
                deadline
            )
            .unwrap(),
            ReleaseStatus::Released
        );
        let image = acquire::acquire(&store, None, deadline).unwrap();
        assert_eq!(
            cleanup::cleanup(&store, &references, "probe-next", image, deadline).unwrap(),
            cleanup::Status::Complete
        );
        assert_eq!(
            std::fs::read_dir(store.root().join("macos-mounts/active"))
                .unwrap()
                .count(),
            0
        );
        println!(
            "durable release verified, same release and original mount never replayed; cleanup and next mount verified"
        );
    } else if release {
        use qq_cli::napcat::install::macos::source::image::mounted::Observation;
        let observed =
            Observation::inspect(recovered.image(), recovered.root(), deadline, &|| Ok(()))
                .unwrap();
        let pins = observed.pin_devices(deadline, &|| Ok(())).unwrap();
        let volume = pins.volume_name().to_owned();
        drop(observed);
        drop(source);
        drop(ready);
        drop(recovered);
        let unmount = std::process::Command::new("/usr/sbin/diskutil")
            .args(["unmount", &volume])
            .output()
            .unwrap();
        assert!(
            unmount.status.success(),
            "normal unmount refused; retain scene"
        );
        pins.revalidate(deadline, &|| Ok(())).unwrap();
        pins.eject_once(deadline, &|| Ok(())).unwrap();
        println!(
            "original raw FD eject accepted after normal volume unmount; driver must verify absence"
        );
    }
    // The external driver owns this test. It must recheck the exact system
    // mapping and normally detach after all host-held readers have exited.
}
fn startup_preparation(base: &Path, image: &Path, archive: &Path) {
    use qq_cli::napcat::{install::macos::source::image::acquire, runtime::macos::start};
    let deadline = Instant::now() + Duration::from_secs(1200);
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
    acquire::acquire(&store, Some(image), deadline).unwrap();
    println!(
        "fixed installation and local image cached; preparing through production start use case"
    );
    let memory = Memory::default();
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &memory,
    };
    let prepared = start::prepare_instance(&context, "startup-probe", deadline).unwrap();
    prepared.verify(&context, deadline).unwrap();
    assert!(prepared.executable().is_file());
    let instance = prepared.runtime().instance_id.clone();
    let run = prepared.runtime().run_id.clone();
    let recovered = start::prepare_instance(&context, "startup-probe", deadline).unwrap();
    recovered.verify(&context, deadline).unwrap();
    assert_eq!(recovered.runtime().instance_id, instance);
    assert_eq!(recovered.runtime().run_id, run);
    assert_eq!(store.read().unwrap().instances[0].state, "prepared");
    assert!(!store.root().join("start-requests").exists());
    assert!(!store.root().join("macos-runs").exists());
    assert_eq!(
        std::fs::read_dir(store.root().join("macos-mounts/active"))
            .unwrap()
            .count(),
        0
    );
    println!(
        "fixed production startup preparation, signed app, original material, read-only recovery and source cleanup verified; QQ was not launched"
    );
}
fn main() {
    for worker in [
        qq_cli::napcat::install::macos::worker_entry as fn() -> Option<u8>,
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
    if matches!(args[1].as_str(), "attach" | "eject" | "release") {
        supervised_mount(
            base,
            Path::new(&args[3]),
            args[1] == "eject",
            args[1] == "release",
        );
        return; // Retain the mounted tree for the external test driver.
    }
    match args[1].as_str() {
        "synthetic" => synthetic(base),
        "image" => image_source(base, Path::new(&args[3])),
        "mounted" => mounted_source(Path::new(&args[3]), Path::new(&args[4])),
        "transition" => transition_source(Path::new(&args[3]), Path::new(&args[4])),
        "source" => original(base, Path::new(&args[3]), Path::new(&args[4])),
        "compare" => {
            let deadline = Instant::now() + Duration::from_secs(180);
            let original = Inventory::inspect(Path::new(&args[3]), deadline).unwrap();
            match original.verify_copy(Path::new(&args[4]), deadline) {
                Ok(_) => println!("copy comparison passed"),
                Err(error) => println!("copy comparison rejected: {}", error.code),
            }
        }
        "prepare" => {
            startup_preparation(base, Path::new(&args[3]), Path::new(&args[4]));
            return; // External driver audits original assets and owns this prepared scene.
        }
        _ => panic!("unknown probe mode"),
    }
    std::fs::remove_dir_all(base).unwrap();
}

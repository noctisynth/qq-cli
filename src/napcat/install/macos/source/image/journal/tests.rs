use super::*;
use std::{cell::Cell, os::unix::fs::PermissionsExt, time::Duration};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    references: References,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
        private_dir(&store.root().join("qq-images")).unwrap();
        let path = store
            .root()
            .join("qq-images")
            .join(format!("{}.dmg", record::hash(b"synthetic")));
        std::fs::write(&path, b"synthetic").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let references = References::with_key(store.root().join("references"), [72; 32]);
        Self {
            _temp: temp,
            store,
            references,
            path,
        }
    }
    fn image(&self) -> Image {
        Image::inspect_expected(&self.path, 9, &record::hash(b"synthetic"), deadline()).unwrap()
    }
    fn run(
        &self,
        request: &str,
        action: impl FnOnce(&Path, &Path, &dyn Fn() -> Result<()>) -> Result<()>,
    ) -> Result<Status> {
        attach(
            &self.store,
            &self.references,
            request,
            self.image(),
            deadline(),
            action,
        )
    }
}
#[test]
fn failed_noop_and_panicked_effects_never_replay() {
    for kind in 0..3 {
        let f = Fixture::new();
        let called = Cell::new(0);
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f.run("request-a", |_, _, guard| {
                guard()?;
                called.set(called.get() + 1);
                match kind {
                    0 => Err(conflict()),
                    1 => Ok(()),
                    _ => panic!("synthetic interruption"),
                }
            })
        }));
        if kind == 2 {
            assert!(first.is_err());
        } else {
            assert!(matches!(first.unwrap().unwrap(), Status::Unknown));
        }
        assert_eq!(called.get(), 1);
        assert!(matches!(
            f.run("request-a", |_, _, _| panic!("replayed")).unwrap(),
            Status::Unknown
        ));
        assert!(
            f.run("request-b", |_, _, _| panic!("second request"))
                .is_err()
        );
    }
}
#[test]
fn original_parent_and_signed_record_are_required() {
    let f = Fixture::new();
    assert!(
        f.run("request-a", |_, mount, guard| {
            let parent = mount.parent().unwrap();
            std::fs::rename(parent, parent.with_extension("old")).unwrap();
            private_dir(parent).unwrap();
            assert!(guard().is_err());
            Err(conflict())
        })
        .is_err()
    );
    // Lost authority retains Pending, which can never redispatch.
    assert!(matches!(
        f.run("request-a", |_, _, _| panic!("replayed")).unwrap(),
        Status::Unknown
    ));
    let references = f.store.root().join("references");
    for entry in std::fs::read_dir(references).unwrap() {
        std::fs::write(entry.unwrap().path(), b"{}").unwrap();
    }
    assert!(f.run("request-a", |_, _, _| panic!("tampered")).is_err());
}
#[test]
fn unclaimed_mount_directory_is_not_adopted() {
    let f = Fixture::new();
    let input = Input::inspect(&f.store, &f.image()).unwrap();
    let parent = record::parent(
        &f.store.root().join("macos-mounts/active"),
        &input,
        "request-a",
        2,
    )
    .unwrap();
    private_dir(&parent).unwrap();
    assert!(f.run("request-a", |_, _, _| panic!("unclaimed")).is_err());
    assert!(parent.exists());
}

#[test]
fn synthetic_mount_completes_recovers_and_rejects_detached_receipt() {
    use std::ffi::OsStr;
    let base = tempfile::tempdir().unwrap().keep().canonicalize().unwrap();
    let source = base.join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("original.txt"), b"original fixture").unwrap();
    let dmg = base.join("fixture.dmg");
    let run = |args: &[&OsStr], guard: &dyn Fn() -> Result<()>| {
        mounted::command::run("/usr/bin/hdiutil", args, None, deadline(), guard)
    };
    run(
        &[
            OsStr::new("create"),
            OsStr::new("-srcfolder"),
            source.as_os_str(),
            OsStr::new("-fs"),
            OsStr::new("HFS+"),
            OsStr::new("-format"),
            OsStr::new("UDZO"),
            dmg.as_os_str(),
        ],
        &|| Ok(()),
    )
    .unwrap();
    let bytes = std::fs::read(&dmg).unwrap();
    let digest = record::hash(&bytes);
    let store = Store::new(base.join("state"));
    let cache = store.root().join("qq-images");
    private_dir(&cache).unwrap();
    let path = cache.join(format!("{digest}.dmg"));
    std::fs::rename(&dmg, &path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let references = References::with_key(store.root().join("references"), [73; 32]);
    let inspect =
        || Image::inspect_expected(&path, bytes.len() as u64, &digest, deadline()).unwrap();
    let mount_path = std::cell::RefCell::new(None);
    let result = attach(
        &store,
        &references,
        "synthetic-mount",
        inspect(),
        deadline(),
        |image, mount, guard| {
            guard()?;
            *mount_path.borrow_mut() = Some(mount.to_path_buf());
            run(
                &[
                    OsStr::new("attach"),
                    OsStr::new("-readonly"),
                    OsStr::new("-nobrowse"),
                    OsStr::new("-noautoopen"),
                    OsStr::new("-mountpoint"),
                    mount.as_os_str(),
                    OsStr::new("-plist"),
                    image.as_os_str(),
                ],
                guard,
            )?;
            Ok(())
        },
    );
    let checked = (|| -> Result<()> {
        let Status::Ready(ready) = result.as_ref().map_err(|_| conflict())? else {
            return Err(conflict());
        };
        assert_eq!(
            std::fs::read(ready.root().join("original.txt")).unwrap(),
            b"original fixture"
        );
        let recovered = attach(
            &store,
            &references,
            "synthetic-mount",
            inspect(),
            deadline(),
            |_, _, _| panic!("replay"),
        )?;
        let Status::Ready(recovered) = recovered else {
            return Err(conflict());
        };
        recovered.revalidate(deadline(), &|| Ok(()))?;
        assert!(
            attach(
                &store,
                &references,
                "different-request",
                inspect(),
                deadline(),
                |_, _, _| panic!("duplicate")
            )
            .is_err()
        );
        Ok(())
    })();
    // Release all root readers before normal detach of this test-owned mount.
    drop(result);
    let mount = mount_path.into_inner().unwrap();
    run(&[OsStr::new("detach"), mount.as_os_str()], &|| Ok(())).unwrap();
    checked.unwrap();
    assert!(
        attach(
            &store,
            &references,
            "synthetic-mount",
            inspect(),
            deadline(),
            |_, _, _| panic!("detached replay")
        )
        .is_err()
    );
    std::fs::remove_dir_all(base).unwrap();
}

const INTERRUPT_FIXTURE: &str =
    "napcat::install::macos::source::image::journal::tests::interrupted_fixture";
#[test]
fn interrupted_fixture() {
    let Some(root) = std::env::var_os("QQ_CLI_MOUNT_JOURNAL_FIXTURE") else {
        return;
    };
    let store = Store::new(root.into());
    let references = References::with_key(store.root().join("references"), [72; 32]);
    let path = store
        .root()
        .join("qq-images")
        .join(format!("{}.dmg", record::hash(b"synthetic")));
    let image = Image::inspect_expected(&path, 9, &record::hash(b"synthetic"), deadline()).unwrap();
    let _ = attach(
        &store,
        &references,
        "request-a",
        image,
        deadline(),
        |_, _, guard| {
            guard()?;
            std::fs::write(store.root().join("entered"), b"pending saved").unwrap();
            loop {
                guard()?;
                std::thread::sleep(Duration::from_millis(5));
            }
        },
    );
}
#[test]
fn separate_process_contention_and_abrupt_death_preserve_no_replay() {
    use std::process::{Command, Stdio};
    let f = Fixture::new();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", INTERRUPT_FIXTURE, "--nocapture"])
        .env("QQ_CLI_MOUNT_JOURNAL_FIXTURE", f.store.root())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let end = deadline();
    while !f.store.root().join("entered").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "fixture exited before pending"
        );
        assert!(Instant::now() < end, "fixture did not enter effect");
        std::thread::sleep(Duration::from_millis(5));
    }
    let concurrent = attach(
        &f.store,
        &f.references,
        "request-b",
        f.image(),
        Instant::now() + Duration::from_millis(100),
        |_, _, _| panic!("concurrent effect"),
    );
    assert!(matches!(concurrent, Err(e) if e.code == "QQ_MOUNT_BUSY"));
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(matches!(
        f.run("request-a", |_, _, _| panic!("interrupted replay"))
            .unwrap(),
        Status::Unknown
    ));
    assert!(
        f.run("request-b", |_, _, _| panic!("unreleased mount"))
            .is_err()
    );
}

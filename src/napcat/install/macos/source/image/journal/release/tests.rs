use super::*;
use std::{ffi::OsStr, os::unix::fs::PermissionsExt, time::Duration};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}
fn run(program: &str, args: &[&OsStr]) -> Result<()> {
    mounted::command::run(program, args, None, deadline(), &|| Ok(())).map(|_| ())
}
struct Fixture {
    base: PathBuf,
    store: Store,
    references: References,
    image: PathBuf,
    mount: PathBuf,
    digest: String,
    size: u64,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir().unwrap().keep().canonicalize().unwrap();
        let source = base.join("source");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("synthetic.txt"), b"original release fixture").unwrap();
        let image = base.join("fixture.dmg");
        run(
            "/usr/bin/hdiutil",
            &[
                OsStr::new("create"),
                OsStr::new("-srcfolder"),
                source.as_os_str(),
                OsStr::new("-fs"),
                OsStr::new("HFS+"),
                OsStr::new("-format"),
                OsStr::new("UDZO"),
                image.as_os_str(),
            ],
        )
        .unwrap();
        let bytes = std::fs::read(&image).unwrap();
        let digest = record::hash(&bytes);
        let store = Store::new(base.join("state"));
        let cache = store.root().join("qq-images");
        private_dir(&cache).unwrap();
        let path = cache.join(format!("{digest}.dmg"));
        std::fs::rename(image, &path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let references = References::with_key(store.root().join("references"), [75; 32]);
        let image =
            Image::inspect_expected(&path, bytes.len() as u64, &digest, deadline()).unwrap();
        let result = super::super::attach(
            &store,
            &references,
            "mount-a",
            image,
            deadline(),
            |image, mount, guard| {
                mounted::command::run(
                    "/usr/bin/hdiutil",
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
                    None,
                    deadline(),
                    guard,
                )
                .map(|_| ())
            },
        )
        .unwrap();
        let super::super::Status::Ready(ready) = result else {
            panic!("mount not ready");
        };
        let mount = ready.root().to_path_buf();
        let image = (*ready).into_image();
        drop(image);
        Self {
            base,
            store,
            references,
            image: path,
            mount,
            digest,
            size: bytes.len() as u64,
        }
    }
    fn image(&self) -> Image {
        Image::inspect_expected(&self.image, self.size, &self.digest, deadline()).unwrap()
    }
    fn await_released(&self, status: Status) -> Status {
        self.await_released_for("mount-a", "release-a", status)
    }
    fn await_released_for(&self, mount_request: &str, request: &str, mut status: Status) -> Status {
        let end = deadline();
        // A successful eject is not synchronous disappearance proof. Only
        // observe the original journal/device generation; never retry effects.
        while status == Status::Unknown {
            assert!(Instant::now() < end, "original devices did not disappear");
            std::thread::sleep(Duration::from_millis(50));
            status = release(
                &self.store,
                &self.references,
                mount_request,
                request,
                self.image(),
                end,
                |_, _| panic!("release effects were replayed while awaiting disappearance"),
            )
            .unwrap();
        }
        status
    }
    fn release_for_cleanup(&self) {
        let status = self.release("release-a", effects).unwrap();
        assert_eq!(self.await_released(status), Status::Released);
    }
    fn release(
        &self,
        request: &str,
        action: impl FnOnce(mounted::Devices, &dyn Fn() -> Result<()>) -> Result<()>,
    ) -> Result<Status> {
        release(
            &self.store,
            &self.references,
            "mount-a",
            request,
            self.image(),
            deadline(),
            action,
        )
    }
}
fn effects(pins: mounted::Devices, guard: &dyn Fn() -> Result<()>) -> Result<()> {
    let volume = pins.volume_name().to_owned();
    mounted::command::run(
        "/usr/sbin/diskutil",
        &[OsStr::new("unmount"), OsStr::new(&volume)],
        None,
        deadline(),
        guard,
    )
    .expect("synthetic volume unmount must succeed");
    pins.eject_once(deadline(), guard)
        .expect("synthetic device eject must succeed");
    Ok(())
}
#[test]
fn complete_failed_and_interrupted_release_never_replay_and_recover_by_absence() {
    for mode in 0..5 {
        let f = Fixture::new();
        let called = std::cell::Cell::new(0);
        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            f.release("release-a", |pins, guard| {
                called.set(called.get() + 1);
                match mode {
                    0 => effects(pins, guard),
                    1 => Ok(()), // A successful callback without effects cannot prove release.
                    2 => Err(conflict()),
                    3 => panic!("synthetic interruption before effect"),
                    _ => {
                        effects(pins, guard)?;
                        panic!("synthetic interruption after effect");
                    }
                }
            })
        }));
        if mode >= 3 {
            assert!(first.is_err());
        } else {
            assert_eq!(
                if mode == 0 {
                    f.await_released(first.unwrap().unwrap())
                } else {
                    first.unwrap().unwrap()
                },
                if mode == 0 {
                    Status::Released
                } else {
                    Status::Unknown
                }
            );
        }
        assert_eq!(called.get(), 1);
        assert!(
            f.release("release-b", |_, _| panic!("different release request"))
                .is_err()
        );
        let observed = f
            .release("release-a", |_, _| panic!("replayed release"))
            .unwrap();
        let observed = if mode == 0 || mode == 4 {
            f.await_released(observed)
        } else {
            observed
        };
        assert_eq!(
            observed,
            if mode == 0 || mode == 4 {
                Status::Released
            } else {
                Status::Unknown
            }
        );
        if observed == Status::Unknown {
            // Explicit fixture-owned cleanup only, outside the production journal.
            run(
                "/usr/bin/hdiutil",
                &[OsStr::new("detach"), f.mount.as_os_str()],
            )
            .unwrap();
        }
        assert_eq!(
            f.release("release-a", |_, _| panic!("replayed after disappearance"))
                .unwrap(),
            Status::Released
        );
        let old = super::super::attach(
            &f.store,
            &f.references,
            "mount-a",
            f.image(),
            deadline(),
            |_, _, _| panic!("old mount replay"),
        );
        assert!(matches!(old.unwrap(), super::super::Status::Released));
        assert!(!f.mount.join("synthetic.txt").exists());
        std::fs::remove_dir_all(f.base).unwrap();
    }
}

mod process;

#[test]
fn already_disappeared_completed_mount_releases_without_dispatch() {
    let f = Fixture::new();
    run(
        "/usr/bin/hdiutil",
        &[OsStr::new("detach"), f.mount.as_os_str()],
    )
    .unwrap();
    assert_eq!(
        f.release("release-a", |_, _| panic!("already absent effect"))
            .unwrap(),
        Status::Released
    );
    assert_eq!(
        f.release("release-a", |_, _| panic!("already released replay"))
            .unwrap(),
        Status::Released
    );
    std::fs::remove_dir_all(f.base).unwrap();
}

mod cleanup;

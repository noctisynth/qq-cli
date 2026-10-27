use super::*;
use std::process::{Command, Stdio};
const FIXTURE: &str =
    "napcat::install::macos::source::image::journal::release::tests::process::fixture";
#[test]
fn fixture() {
    let Ok(mode) = std::env::var("QQ_CLI_RELEASE_FIXTURE") else {
        return;
    };
    let base = PathBuf::from(std::env::var_os("QQ_CLI_RELEASE_BASE").unwrap());
    let digest = std::env::var("QQ_CLI_RELEASE_DIGEST").unwrap();
    let mount = PathBuf::from(std::env::var_os("QQ_CLI_RELEASE_MOUNT").unwrap());
    let store = Store::new(base.join("state"));
    let references = References::with_key(store.root().join("references"), [75; 32]);
    let image = store.root().join("qq-images").join(format!("{digest}.dmg"));
    let size = std::fs::metadata(&image).unwrap().len();
    let f = Fixture {
        base,
        store,
        references,
        image,
        mount,
        digest,
        size,
    };
    if let Some(target) = mode.strip_prefix("cleanup-") {
        let target: usize = target.parse().unwrap();
        let count = std::cell::Cell::new(0);
        let _ = super::super::super::cleanup::perform(
            &f.store,
            &f.references,
            "mount-a",
            f.image(),
            deadline(),
            &|| {
                count.set(count.get() + 1);
                if count.get() == target {
                    std::fs::write(
                        f.store.root().join("cleanup-entered"),
                        b"cleanup checkpoint",
                    )
                    .unwrap();
                    loop {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                Ok(())
            },
        );
        return;
    }
    let _ = f.release("release-a", |pins, guard| {
        if mode == "after" {
            effects(pins, guard)?;
        }
        guard()?;
        std::fs::write(f.store.root().join("release-entered"), b"durable release").unwrap();
        loop {
            guard()?;
            std::thread::sleep(Duration::from_millis(5));
        }
    });
}
#[test]
fn independent_release_contention_and_abrupt_death_never_redispatch() {
    for mode in ["before", "after"] {
        let f = Fixture::new();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", FIXTURE, "--nocapture"])
            .env("QQ_CLI_RELEASE_FIXTURE", mode)
            .env("QQ_CLI_RELEASE_BASE", &f.base)
            .env("QQ_CLI_RELEASE_DIGEST", &f.digest)
            .env("QQ_CLI_RELEASE_MOUNT", &f.mount)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let end = deadline();
        while !f.store.root().join("release-entered").exists() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "fixture exited before durable release"
            );
            assert!(Instant::now() < end, "fixture did not enter release");
            std::thread::sleep(Duration::from_millis(5));
        }
        let concurrent = release(
            &f.store,
            &f.references,
            "mount-a",
            "release-b",
            f.image(),
            Instant::now() + Duration::from_millis(100),
            |_, _| panic!("concurrent release"),
        );
        assert!(matches!(concurrent, Err(e) if e.code == "QQ_MOUNT_BUSY"));
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(
            f.release("release-a", |_, _| panic!("interrupted replay"))
                .unwrap(),
            if mode == "before" {
                Status::Unknown
            } else {
                Status::Released
            }
        );
        if mode == "before" {
            run(
                "/usr/bin/hdiutil",
                &[OsStr::new("detach"), f.mount.as_os_str()],
            )
            .unwrap();
        }
        assert_eq!(
            f.release("release-a", |_, _| panic!("cleanup replay"))
                .unwrap(),
            Status::Released
        );
        std::fs::remove_dir_all(f.base).unwrap();
    }
}

#[test]
fn independent_cleanup_contention_and_death_after_each_rmdir_recover() {
    for target in [2, 4] {
        let f = Fixture::new();
        f.release_for_cleanup();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", FIXTURE, "--nocapture"])
            .env("QQ_CLI_RELEASE_FIXTURE", format!("cleanup-{target}"))
            .env("QQ_CLI_RELEASE_BASE", &f.base)
            .env("QQ_CLI_RELEASE_DIGEST", &f.digest)
            .env("QQ_CLI_RELEASE_MOUNT", &f.mount)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let end = deadline();
        while !f.store.root().join("cleanup-entered").exists() {
            assert!(
                child.try_wait().unwrap().is_none(),
                "cleanup fixture exited early"
            );
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        }
        let concurrent = super::super::super::cleanup::cleanup(
            &f.store,
            &f.references,
            "mount-a",
            f.image(),
            Instant::now() + Duration::from_millis(100),
        );
        assert!(matches!(concurrent, Err(e) if e.code == "QQ_MOUNT_BUSY"));
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(
            super::super::super::cleanup::cleanup(
                &f.store,
                &f.references,
                "mount-a",
                f.image(),
                deadline()
            )
            .unwrap(),
            super::super::super::cleanup::Status::Complete
        );
        assert!(!f.mount.parent().unwrap().exists());
        std::fs::remove_dir_all(f.base).unwrap();
    }
}

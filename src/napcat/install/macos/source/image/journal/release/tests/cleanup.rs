use super::super::super::cleanup as cleaning;
use super::*;
fn clean(f: &Fixture) -> Result<cleaning::Status> {
    cleaning::cleanup(&f.store, &f.references, "mount-a", f.image(), deadline())
}
fn second(f: &Fixture) -> Result<super::super::super::Status> {
    super::super::super::attach(
        &f.store,
        &f.references,
        "mount-b",
        f.image(),
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
}
#[test]
fn cleanup_requires_release_original_empty_directories_and_preserves_legacy_records() {
    let f = Fixture::new();
    assert!(clean(&f).is_err());
    assert!(f.mount.join("synthetic.txt").exists());
    f.release_for_cleanup();
    assert!(second(&f).is_err());
    let extra = f.mount.join("foreign");
    std::fs::write(&extra, b"preserve").unwrap();
    assert!(clean(&f).is_err());
    assert_eq!(std::fs::read(&extra).unwrap(), b"preserve");
    std::fs::remove_file(extra).unwrap();
    let moved = f.mount.with_extension("original");
    std::fs::rename(&f.mount, &moved).unwrap();
    private_dir(&f.mount).unwrap();
    assert!(clean(&f).is_err());
    assert!(f.mount.exists());
    std::fs::remove_dir(&f.mount).unwrap();
    std::os::unix::fs::symlink(&moved, &f.mount).unwrap();
    assert!(clean(&f).is_err());
    assert!(moved.exists());
    std::fs::remove_file(&f.mount).unwrap();
    std::fs::rename(&moved, &f.mount).unwrap();
    let path = f
        .store
        .root()
        .join("macos-mounts/intents")
        .join(format!("{}.json", record::hash(b"mount-a")));
    let original = load(&path, &f.references).unwrap().unwrap();
    let mut legacy = original.clone();
    legacy.version = 1;
    legacy.mount = None;
    let old_parent = record::parent(
        &f.store.root().join("macos-mounts/active"),
        &legacy.input,
        "mount-a",
        1,
    )
    .unwrap();
    let parent = f.mount.parent().unwrap();
    std::fs::rename(parent, &old_parent).unwrap();
    {
        let lock =
            Lock::acquire(f.store.root(), "qq-mount.lock", deadline(), "QQ_MOUNT_BUSY").unwrap();
        let transaction = f.store.transaction(deadline()).unwrap();
        save(&transaction, &lock, &f.references, &path, &legacy).unwrap();
    }
    assert!(clean(&f).is_err());
    assert!(old_parent.join("mount").exists());
    assert_eq!(
        f.release("release-a", |_, _| panic!("legacy replay"))
            .unwrap(),
        Status::Released
    );
    std::fs::rename(&old_parent, parent).unwrap();
    {
        let lock =
            Lock::acquire(f.store.root(), "qq-mount.lock", deadline(), "QQ_MOUNT_BUSY").unwrap();
        let transaction = f.store.transaction(deadline()).unwrap();
        save(&transaction, &lock, &f.references, &path, &original).unwrap();
    }
    assert_eq!(clean(&f).unwrap(), cleaning::Status::Complete);
    assert!(!parent.exists());
    assert!(path.exists());
    std::fs::remove_dir_all(f.base).unwrap();
}
#[test]
fn every_cleanup_boundary_recovers_without_deleting_the_next_mount() {
    let f = Fixture::new();
    f.release_for_cleanup();
    // Each invocation stops after its next persisted phase or deletion. This
    // includes crashes after rmdir but before recording its completion.
    for event in 0..5 {
        let stopped = cleaning::perform(
            &f.store,
            &f.references,
            "mount-a",
            f.image(),
            deadline(),
            &|| Err(conflict()),
        );
        assert!(stopped.is_err(), "event {event}");
        if event < 4 {
            assert!(second(&f).is_err());
        }
    }
    assert_eq!(clean(&f).unwrap(), cleaning::Status::Complete);
    assert!(!f.mount.parent().unwrap().exists());
    let super::super::super::Status::Ready(ready) = second(&f).unwrap() else {
        panic!("new mount not ready");
    };
    let new_mount = ready.root().to_path_buf();
    assert_ne!(new_mount, f.mount);
    assert!(new_mount.join("synthetic.txt").exists());
    assert_eq!(clean(&f).unwrap(), cleaning::Status::Complete);
    assert_eq!(
        f.release("release-a", |_, _| panic!("historical release replay"))
            .unwrap(),
        Status::Released
    );
    assert!(matches!(
        super::super::super::attach(
            &f.store,
            &f.references,
            "mount-a",
            f.image(),
            deadline(),
            |_, _, _| panic!("historical attach replay")
        )
        .unwrap(),
        super::super::super::Status::Released
    ));
    assert!(new_mount.join("synthetic.txt").exists());
    drop(ready);
    assert_eq!(
        f.await_released_for(
            "mount-b",
            "release-b",
            release(
                &f.store,
                &f.references,
                "mount-b",
                "release-b",
                f.image(),
                deadline(),
                effects,
            )
            .unwrap(),
        ),
        Status::Released
    );
    assert_eq!(
        cleaning::cleanup(&f.store, &f.references, "mount-b", f.image(), deadline()).unwrap(),
        cleaning::Status::Complete
    );
    std::fs::remove_dir_all(f.base).unwrap();
}

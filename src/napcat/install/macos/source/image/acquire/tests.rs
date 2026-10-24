use super::*;
use std::time::Duration;
const BYTES: &[u8] = b"synthetic disk image";
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn setup() -> (tempfile::TempDir, Store, String) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let digest = format!("{:x}", Sha256::digest(BYTES));
    (temp, store, digest)
}
#[test]
fn cache_is_verified_without_reopening_source_and_corruption_is_preserved() {
    let (_temp, store, digest) = setup();
    let image = acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
        Ok(Box::new(std::io::Cursor::new(BYTES)))
    })
    .unwrap();
    acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
        panic!("cache reopened source")
    })
    .unwrap();
    std::fs::write(image.path(), b"changed").unwrap();
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| panic!(
            "corrupt cache replaced"
        ))
        .is_err()
    );
    assert_eq!(std::fs::read(image.path()).unwrap(), b"changed");
}
#[test]
fn short_long_wrong_and_failed_streams_never_publish() {
    for bytes in [
        b"short".as_slice(),
        b"synthetic disk image plus".as_slice(),
        b"wrong but same bytes".as_slice(),
    ] {
        let (_temp, store, digest) = setup();
        assert!(
            acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| Ok(
                Box::new(std::io::Cursor::new(bytes.to_owned()))
            ))
            .is_err()
        );
        assert_eq!(
            std::fs::read_dir(store.root().join("qq-images"))
                .unwrap()
                .count(),
            0
        );
    }
    let (_temp, store, digest) = setup();
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| Err(
            invalid()
        ))
        .is_err()
    );
    assert_eq!(
        std::fs::read_dir(store.root().join("qq-images"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn interrupted_staging_is_preserved_and_not_adopted() {
    let (_temp, store, digest) = setup();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
            panic!("interrupted source")
        })
    }));
    assert!(result.is_err());
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| panic!(
            "residue replayed"
        ))
        .is_err()
    );
    assert_eq!(
        std::fs::read_dir(store.root().join("qq-images"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn target_appearing_during_transfer_is_never_overwritten() {
    let (_temp, store, digest) = setup();
    let target = store.root().join("qq-images").join(format!("{digest}.dmg"));
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
            std::fs::write(&target, b"preserve destination").unwrap();
            Ok(Box::new(std::io::Cursor::new(BYTES)))
        })
        .is_err()
    );
    assert_eq!(std::fs::read(target).unwrap(), b"preserve destination");
    assert_eq!(
        std::fs::read_dir(store.root().join("qq-images"))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn replaced_parent_prevents_writes_and_cleanup_of_foreign_files() {
    let (_temp, store, digest) = setup();
    let root = store.root().join("qq-images");
    let old = store.root().join("old-images");
    let partial = format!("{digest}.partial");
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
            std::fs::rename(&root, &old).unwrap();
            private_dir(&root).unwrap();
            std::fs::write(root.join(&partial), b"foreign").unwrap();
            Ok(Box::new(std::io::Cursor::new(BYTES)))
        })
        .is_err()
    );
    assert_eq!(std::fs::read(root.join(&partial)).unwrap(), b"foreign");
    assert_eq!(std::fs::metadata(old.join(&partial)).unwrap().len(), 0);
}

struct InterruptedAfterBytes(bool);
impl Read for InterruptedAfterBytes {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.0 {
            panic!("interrupted before final validation")
        }
        self.0 = true;
        buffer[..BYTES.len()].copy_from_slice(BYTES);
        Ok(BYTES.len())
    }
}
#[test]
fn fully_downloaded_residue_recovers_without_source_or_replacement() {
    let (_temp, store, digest) = setup();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| acquire_with(
            &store,
            BYTES.len() as u64,
            &digest,
            deadline(),
            |_| Ok(Box::new(InterruptedAfterBytes(false)))
        )))
        .is_err()
    );
    let stage = store
        .root()
        .join("qq-images")
        .join(format!("{digest}.partial"));
    let inode = std::fs::metadata(&stage).unwrap().ino();
    let recovered = acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
        panic!("must not fetch again")
    })
    .unwrap();
    assert!(!stage.exists());
    assert_eq!(std::fs::metadata(recovered.path()).unwrap().ino(), inode);
    recovered.revalidate(deadline()).unwrap();
}
#[test]
fn changed_complete_residue_is_not_deleted_or_replaced() {
    let (_temp, store, digest) = setup();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| acquire_with(
            &store,
            BYTES.len() as u64,
            &digest,
            deadline(),
            |_| Ok(Box::new(InterruptedAfterBytes(false)))
        )))
        .is_err()
    );
    let stage = store
        .root()
        .join("qq-images")
        .join(format!("{digest}.partial"));
    std::fs::write(&stage, b"changed").unwrap();
    assert!(
        acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| panic!(
            "must not replace residue"
        ))
        .is_err()
    );
    assert_eq!(std::fs::read(&stage).unwrap(), b"changed");
}

#[test]
fn local_quarantine_survives_publication_and_interrupted_recovery() {
    for interrupted in [false, true] {
        let (temp, store, digest) = setup();
        let source = temp.path().canonicalize().unwrap().join("source.dmg");
        std::fs::write(&source, BYTES).unwrap();
        let label = b"0083;00000000;qq-cli-synthetic;";
        rustix::fs::setxattr(
            &source,
            "com.apple.quarantine",
            label,
            rustix::fs::XattrFlags::empty(),
        )
        .unwrap();
        let original =
            Image::inspect_expected(&source, BYTES.len() as u64, &digest, deadline()).unwrap();
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            acquire_with(
                &store,
                BYTES.len() as u64,
                &digest,
                deadline(),
                |destination| {
                    quarantine::copy(&original, destination, deadline())?;
                    if interrupted {
                        Ok(Box::new(InterruptedAfterBytes(false)) as Box<dyn Read>)
                    } else {
                        let image = Image::inspect_expected(
                            &source,
                            BYTES.len() as u64,
                            &digest,
                            deadline(),
                        )?;
                        Ok(Box::new(LocalInput {
                            image,
                            deadline: deadline(),
                        }) as Box<dyn Read>)
                    }
                },
            )
        }));
        if interrupted {
            assert!(attempt.is_err());
        } else {
            attempt.unwrap().unwrap();
        }
        let cached = acquire_with(&store, BYTES.len() as u64, &digest, deadline(), |_| {
            panic!("must not reopen source")
        })
        .unwrap();
        assert_eq!(
            quarantine::read(&cached.file).unwrap().as_deref(),
            Some(label.as_slice())
        );
        original.revalidate(deadline()).unwrap();
        rustix::fs::setxattr(
            cached.path(),
            "com.apple.quarantine",
            b"changed",
            rustix::fs::XattrFlags::empty(),
        )
        .unwrap();
        assert!(cached.revalidate(deadline()).is_err());
    }
}

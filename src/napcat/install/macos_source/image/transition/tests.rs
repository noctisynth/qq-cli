use super::*;
use std::{os::unix::fs::PermissionsExt, time::Duration};

const BYTES: &[u8] = b"synthetic fixed disk image";
const CACHE: &str = "com.apple.diskimages.recentcksum";
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn set(file: &File, key: &str, bytes: &[u8]) {
    rustix::fs::fsetxattr(file, key, bytes, rustix::fs::XattrFlags::empty()).unwrap();
}
fn fixture() -> (tempfile::TempDir, Image) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap().join("fixture.dmg");
    std::fs::write(&path, BYTES).unwrap();
    let file = open(&path).unwrap();
    set(
        &file,
        "com.apple.quarantine",
        b"0083;00000000;qq-cli-synthetic;",
    );
    set(&file, "org.qq-cli.synthetic", b"retain");
    let image = Image::inspect_expected(
        &path,
        BYTES.len() as u64,
        &format!("{:x}", Sha256::digest(BYTES)),
        deadline(),
    )
    .unwrap();
    (temp, image)
}
#[test]
fn checksum_cache_addition_update_and_no_change_preserve_strict_followup() {
    for initial in [false, true] {
        let (_temp, image) = fixture();
        if initial {
            set(&image.file, CACHE, b"synthetic old cache");
        }
        let image =
            Image::inspect_expected(image.path(), BYTES.len() as u64, &image.digest, deadline())
                .unwrap();
        let held = image.file.try_clone().unwrap();
        let path = image.path().to_owned();
        let old =
            Image::inspect_expected(&path, BYTES.len() as u64, &image.digest, deadline()).unwrap();
        let transition = MountTransition::prepare(image, deadline(), &|| Ok(())).unwrap();
        assert_eq!(transition.path(), path);
        transition.verify_before(deadline(), &|| Ok(())).unwrap();
        set(&held, CACHE, b"synthetic updated cache");
        assert!(transition.verify_before(deadline(), &|| Ok(())).is_err());
        assert!(old.revalidate(deadline()).is_err());
        let image = transition.finish(deadline(), &|| Ok(())).unwrap();
        image.revalidate(deadline()).unwrap();
        let image = MountTransition::prepare(image, deadline(), &|| Ok(()))
            .unwrap()
            .finish(deadline(), &|| Ok(()))
            .unwrap();
        set(&held, CACHE, b"changed after completion");
        assert!(image.revalidate(deadline()).is_err());
    }
}
#[test]
fn all_other_metadata_and_original_identity_changes_are_rejected_without_repair() {
    for change in 0..13 {
        let (_temp, image) = fixture();
        set(&image.file, CACHE, b"existing cache");
        let image =
            Image::inspect_expected(image.path(), BYTES.len() as u64, &image.digest, deadline())
                .unwrap();
        let held = image.file.try_clone().unwrap();
        let path = image.path().to_owned();
        let transition = MountTransition::prepare(image, deadline(), &|| Ok(())).unwrap();
        match change {
            0 => set(&held, "com.apple.quarantine", b"changed"),
            1 => set(&held, "org.qq-cli.synthetic", b"changed"),
            2 => set(&held, "org.qq-cli.new", b"unexpected"),
            3 => rustix::fs::fremovexattr(&held, CACHE).unwrap(),
            4 => held
                .set_permissions(std::fs::Permissions::from_mode(0o400))
                .unwrap(),
            5 => {
                std::fs::rename(&path, path.with_extension("original")).unwrap();
                std::fs::write(&path, BYTES).unwrap();
            }
            6 => std::fs::write(&path, b"changed bytes").unwrap(),
            7 => std::fs::hard_link(&path, path.with_extension("link")).unwrap(),
            8 => set(&held, CACHE, &[b'x'; 4097]),
            9 => held
                .set_times(
                    std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH),
                )
                .unwrap(),
            10 => rustix::fs::fremovexattr(&held, "org.qq-cli.synthetic").unwrap(),
            11 => assert!(
                std::process::Command::new("/bin/chmod")
                    .args(["+a", "everyone allow read"])
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            ),
            _ => assert!(
                std::process::Command::new("/usr/bin/chflags")
                    .arg("hidden")
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            ),
        }
        assert!(
            transition.finish(deadline(), &|| Ok(())).is_err(),
            "change {change}"
        );
        if change == 0 {
            assert_eq!(
                quarantine::read(&held).unwrap().as_deref(),
                Some(b"changed".as_slice())
            );
        }
        if change == 6 {
            assert_eq!(std::fs::read(&path).unwrap(), b"changed bytes");
        }
    }
}
#[test]
fn deadline_and_lost_guard_do_not_refresh_old_observation() {
    let (_temp, image) = fixture();
    assert!(MountTransition::prepare(image, deadline(), &|| Err(invalid())).is_err());
    for expired in [false, true] {
        let (_temp, image) = fixture();
        let transition = MountTransition::prepare(image, deadline(), &|| Ok(())).unwrap();
        assert!(
            transition
                .verify_before(deadline(), &|| Err(invalid()))
                .is_err()
        );
        assert!(
            transition
                .verify_before(Instant::now(), &|| Ok(()))
                .is_err()
        );
        let result = if expired {
            transition.finish(Instant::now(), &|| Ok(()))
        } else {
            transition.finish(deadline(), &|| Err(invalid()))
        };
        assert!(result.is_err());
    }
}

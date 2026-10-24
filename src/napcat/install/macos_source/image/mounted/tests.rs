use super::*;
use sha2::{Digest, Sha256};
use std::{
    ffi::{OsStr, OsString},
    time::Duration,
};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(60)
}
fn hdiutil(args: &[OsString]) -> Result<Vec<u8>> {
    command::run(
        "/usr/bin/hdiutil",
        &args.iter().map(OsString::as_os_str).collect::<Vec<_>>(),
        None,
        deadline(),
        &|| Ok(()),
    )
}
fn attach(path: &Path, mount: &Path) {
    hdiutil(&[
        "attach".into(),
        "-readonly".into(),
        "-nobrowse".into(),
        "-noautoopen".into(),
        "-mountpoint".into(),
        mount.into(),
        "-plist".into(),
        path.into(),
    ])
    .unwrap();
}
fn detach(path: &Path, mount: &Path) -> report::Selected {
    let current =
        report::select(&command::info(deadline(), &|| Ok(())).unwrap(), path, mount).unwrap();
    hdiutil(&[OsStr::new("detach").into(), current.whole.clone().into()]).unwrap();
    current
}
#[test]
fn live_synthetic_mount_is_read_only_bound_and_detachment_prevents_new_observation() {
    // Keep a failed fixture for inspection; never let TempDir recursively walk
    // a mounted tree during unwinding. Only successful normal detach permits cleanup.
    let base = tempfile::tempdir().unwrap().keep().canonicalize().unwrap();
    let source = base.join("source");
    let mount = base.join("mount");
    let path = base.join("fixture.dmg");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(source.join("subdirectory")).unwrap();
    std::fs::write(
        source.join("synthetic.txt"),
        b"original synthetic mount content",
    )
    .unwrap();
    std::fs::create_dir(&mount).unwrap();
    hdiutil(&[
        "create".into(),
        "-srcfolder".into(),
        source.into_os_string(),
        "-fs".into(),
        "HFS+".into(),
        "-volname".into(),
        "qq-cli-synthetic".into(),
        "-format".into(),
        "UDZO".into(),
        path.clone().into_os_string(),
    ])
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let before = Image::inspect_expected(&path, bytes.len() as u64, &digest, deadline()).unwrap();
    let transition =
        super::super::transition::MountTransition::prepare(before, deadline(), &|| Ok(())).unwrap();
    transition.verify_before(deadline(), &|| Ok(())).unwrap();
    attach(&path, &mount);
    // Authenticate the mount-time checksum-cache transition; the old strict
    // Image check is not relaxed. This test alone owns its attach operation.
    let transitioned = transition.finish(deadline(), &|| Ok(()));
    if transitioned.is_err() {
        detach(&path, &mount);
    }
    let image = transitioned.unwrap();
    let inspected = Observation::inspect(&image, &mount, deadline(), &|| Ok(()));
    let checks = (|| -> Result<()> {
        let observed = inspected.as_ref().map_err(|_| invalid())?;
        observed.revalidate(deadline(), &|| Ok(()))?;
        if observed.root() != mount
            || std::fs::read(mount.join("synthetic.txt")).map_err(|_| invalid())?
                != b"original synthetic mount content"
        {
            return Err(invalid());
        }
        if Observation::inspect(&image, &mount.join("subdirectory"), deadline(), &|| Ok(())).is_ok()
            || observed.revalidate(deadline(), &|| Err(invalid())).is_ok()
        {
            return Err(invalid());
        }
        Ok(())
    })();
    // Explicit test-created fixture, selected again immediately before normal
    // detach. This is test cleanup, not a production ownership inference.
    let original_generation = inspected
        .as_ref()
        .ok()
        .map(|observed| observed.media.clone());
    // Held directory handles deliberately pin the volume. Release our reader
    // before normal detach; never turn a busy volume into forced cleanup.
    drop(inspected);
    let original_mapping = detach(image.path(), &mount);
    assert!(Observation::inspect(&image, &mount, deadline(), &|| Ok(())).is_err());
    assert!(!mount.join("synthetic.txt").exists());
    image.revalidate(deadline()).unwrap();
    checks.unwrap();

    // Attach the very same bytes at the same path again. Device names may be
    // reused, but the registry objects must not authenticate the former attach.
    let transition =
        super::super::transition::MountTransition::prepare(image, deadline(), &|| Ok(())).unwrap();
    transition.verify_before(deadline(), &|| Ok(())).unwrap();
    attach(&path, &mount);
    let transitioned = transition.finish(deadline(), &|| Ok(()));
    if transitioned.is_err() {
        detach(&path, &mount);
    }
    let image = transitioned.unwrap();
    let mut inspected = Observation::inspect(&image, &mount, deadline(), &|| Ok(()));
    let reattached = (|| -> Result<()> {
        let observed = inspected.as_mut().map_err(|_| invalid())?;
        let original_generation = original_generation.ok_or_else(invalid)?;
        if observed.media == original_generation {
            return Err(invalid());
        }
        eprintln!(
            "synthetic BSD device name reused: {}",
            original_mapping.device == observed.selected.device
        );
        observed.revalidate(deadline(), &|| Ok(()))?;
        observed.media = original_generation;
        if observed.revalidate(deadline(), &|| Ok(())).is_ok() {
            return Err(invalid());
        }
        Ok(())
    })();
    drop(inspected);
    detach(image.path(), &mount);
    assert!(!mount.join("synthetic.txt").exists());
    std::fs::remove_dir_all(base).unwrap();
    reattached.unwrap();
}

#[test]
fn ordinary_directory_cannot_impersonate_a_mount_root() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap();
    let bytes = serde_json::to_vec(&serde_json::json!({"images":[{
        "image-path":"/synthetic.dmg", "owner-uid":rustix::process::geteuid().as_raw(),
        "writeable":false,"image-encrypted":false,"system-entities":[
            {"dev-entry":"/dev/disk42","mount-point":path}
        ]
    }]}))
    .unwrap();
    let selected = report::select(&bytes, Path::new("/synthetic.dmg"), &path).unwrap();
    assert!(verify_filesystem(&open_root(&path).unwrap(), &selected).is_err());
}

#[test]
fn original_raw_devices_reject_busy_and_eject_only_after_normal_unmount() {
    let base = tempfile::tempdir().unwrap().keep().canonicalize().unwrap();
    let source = base.join("source");
    let mount = base.join("mount");
    let path = base.join("fixture.dmg");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("synthetic.txt"), b"original eject fixture").unwrap();
    std::fs::create_dir(&mount).unwrap();
    hdiutil(&[
        "create".into(),
        "-srcfolder".into(),
        source.into_os_string(),
        "-fs".into(),
        "HFS+".into(),
        "-format".into(),
        "UDZO".into(),
        path.clone().into_os_string(),
    ])
    .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    attach(&path, &mount);
    let image = Image::inspect_expected(&path, bytes.len() as u64, &digest, deadline()).unwrap();
    let observed = Observation::inspect(&image, &mount, deadline(), &|| Ok(())).unwrap();
    let pins = observed.pin_devices(deadline(), &|| Ok(())).unwrap();
    assert_eq!(
        pins.eject_once(deadline(), &|| Ok(())).unwrap_err().code,
        "QQ_MEDIA_BUSY"
    );
    observed.revalidate(deadline(), &|| Ok(())).unwrap();
    let selection = media::inspect(&observed.selected, deadline(), &|| Ok(())).unwrap();
    assert!(
        crate::native::macos_disk::Disk::open(
            &observed.selected.whole,
            selection.whole.0,
            selection.whole.1.wrapping_add(1)
        )
        .is_err()
    );
    let pins = observed.pin_devices(deadline(), &|| Ok(())).unwrap();
    assert!(pins.eject_once(deadline(), &|| Err(invalid())).is_err());
    observed.revalidate(deadline(), &|| Ok(())).unwrap();
    let pins = observed.pin_devices(deadline(), &|| Ok(())).unwrap();
    let volume = pins.volume_name().to_owned();
    drop(observed);
    // Original fixture only. Keep raw FD pins across ordinary volume unmount.
    command::run(
        "/usr/sbin/diskutil",
        &[OsStr::new("unmount"), OsStr::new(&volume)],
        None,
        deadline(),
        &|| Ok(()),
    )
    .unwrap();
    pins.revalidate(deadline(), &|| Ok(())).unwrap();
    pins.eject_once(deadline(), &|| Ok(())).unwrap();
    // Eject return alone is insufficient; independently require mapping absence.
    require_absent(image.path(), &mount, deadline(), &|| Ok(())).unwrap();
    assert!(!mount.join("synthetic.txt").exists());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir_all(base).unwrap();
}

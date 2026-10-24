use super::*;
use std::time::Duration;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn unchanged_nodes_are_not_rehashed_but_changed_nodes_match_full_observation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("Synthetic.app");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("large"), vec![7; 1024 * 1024]).unwrap();
    std::fs::write(root.join("small"), b"before").unwrap();
    let original = Inventory::inspect(&root, deadline()).unwrap();
    assert_eq!(original.content_bytes_read, 1024 * 1024 + 6);
    let cached = Inventory::inspect_using(&root, deadline(), Some(&original)).unwrap();
    assert_eq!(cached.content_bytes_read, 0);
    assert_eq!(cached.fingerprint(), original.fingerprint());
    let old_time = std::fs::metadata(root.join("small"))
        .unwrap()
        .modified()
        .unwrap();
    std::fs::write(root.join("small"), b"change").unwrap();
    File::options()
        .write(true)
        .open(root.join("small"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(old_time))
        .unwrap();
    let changed = Inventory::inspect_using(&root, deadline(), Some(&cached)).unwrap();
    assert_eq!(changed.content_bytes_read, 6);
    let full = Inventory::inspect(&root, deadline()).unwrap();
    assert!(changed.entries == full.entries && changed.stamps == full.stamps);
    assert_eq!(changed.fingerprint(), full.fingerprint());
    assert_ne!(changed.fingerprint(), original.fingerprint());
}

#[test]
fn cached_walk_still_enumerates_nodes_and_rechecks_xattrs_and_replacements() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("Synthetic.app");
    std::fs::create_dir(&root).unwrap();
    let file = root.join("data");
    std::fs::write(&file, b"unchanged bytes").unwrap();
    let original = Inventory::inspect(&root, deadline()).unwrap();
    rustix::fs::setxattr(
        &file,
        "org.qq-cli.synthetic",
        b"new",
        rustix::fs::XattrFlags::empty(),
    )
    .unwrap();
    std::fs::write(root.join("added"), b"new file").unwrap();
    let changed = Inventory::inspect_using(&root, deadline(), Some(&original)).unwrap();
    assert_eq!(changed.entry_count(), original.entry_count() + 1);
    assert_ne!(changed.fingerprint(), original.fingerprint());
    let full = Inventory::inspect(&root, deadline()).unwrap();
    assert!(changed.entries == full.entries && changed.stamps == full.stamps);
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, b"unchanged bytes").unwrap();
    std::fs::remove_file(root.join("added")).unwrap();
    let replaced = Inventory::inspect_using(&root, deadline(), Some(&changed)).unwrap();
    assert_eq!(replaced.content_bytes_read, b"unchanged bytes".len() as u64);
    let full = Inventory::inspect(&root, deadline()).unwrap();
    assert!(replaced.entries == full.entries && replaced.stamps == full.stamps);
}

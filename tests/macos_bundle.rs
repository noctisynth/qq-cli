#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::napcat::install::macos_bundle::Inventory;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn fixture(base: &Path) -> PathBuf {
    let root = base.join("Source.app");
    let framework = root.join("Contents/Frameworks/Thing.framework");
    std::fs::create_dir_all(framework.join("Versions/A/Resources")).unwrap();
    std::fs::write(framework.join("Versions/A/Thing"), b"synthetic executable").unwrap();
    std::fs::set_permissions(
        framework.join("Versions/A/Thing"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    std::fs::write(
        framework.join("Versions/A/Resources/Data"),
        b"synthetic resource",
    )
    .unwrap();
    symlink("A", framework.join("Versions/Current")).unwrap();
    symlink("Versions/Current/Thing", framework.join("Thing")).unwrap();
    symlink("Versions/Current/Resources", framework.join("Resources")).unwrap();
    root
}
#[test]
fn inventories_framework_aliases_and_verifies_a_distinct_complete_copy() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let inventory = Inventory::inspect(&source, deadline()).unwrap();
    assert_eq!(inventory.total_bytes(), 38);
    assert_eq!(inventory.entry_count(), 12);
    inventory.revalidate(deadline()).unwrap();
    let copy = base.join("Copy.app");
    let status = std::process::Command::new("/usr/bin/ditto")
        .args(["--rsrc", "--extattr", "--qtn", "--acl"])
        .arg(&source)
        .arg(&copy)
        .status()
        .unwrap();
    assert!(status.success());
    let copied = inventory.verify_copy(&copy, deadline()).unwrap();
    assert_eq!(copied.fingerprint(), inventory.fingerprint());
    assert_eq!(
        std::fs::read_link(copy.join("Contents/Frameworks/Thing.framework/Versions/Current"))
            .unwrap(),
        Path::new("A")
    );
    std::fs::write(copy.join("Contents/extra"), b"unexpected").unwrap();
    assert!(inventory.verify_copy(&copy, deadline()).is_err());
    assert!(inventory.verify_copy(&source, deadline()).is_err());
}
#[test]
fn content_modes_and_original_identity_are_part_of_the_observation() {
    for variant in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = fixture(&base);
        let inventory = Inventory::inspect(&root, deadline()).unwrap();
        let file = root.join("Contents/Frameworks/Thing.framework/Versions/A/Thing");
        match variant {
            0 => std::fs::write(&file, b"changed executable").unwrap(),
            1 => std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap(),
            2 => {
                std::fs::rename(&root, base.join("Old.app")).unwrap();
                fixture(&base);
            }
            _ => std::fs::write(root.join("Contents/extra"), b"extra").unwrap(),
        }
        assert!(inventory.revalidate(deadline()).is_err());
    }
}
#[test]
fn rejects_escaping_absolute_broken_cyclic_hard_and_special_entries() {
    for variant in 0..7 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = fixture(&base);
        let entry = root.join("Contents/bad");
        let mut socket = None;
        match variant {
            0 => symlink("../../../", &entry).unwrap(),
            1 => symlink(&base, &entry).unwrap(),
            2 => symlink("missing", &entry).unwrap(),
            3 => symlink("bad", &entry).unwrap(),
            4 => std::fs::hard_link(
                root.join("Contents/Frameworks/Thing.framework/Versions/A/Thing"),
                &entry,
            )
            .unwrap(),
            5 => {
                socket = Some(std::os::unix::net::UnixListener::bind(&entry).unwrap());
            }
            _ => {
                std::fs::create_dir(&entry).unwrap();
                std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o777)).unwrap();
            }
        }
        assert!(Inventory::inspect(&root, deadline()).is_err());
        drop(socket);
    }
}
#[test]
fn rejects_alias_roots_depth_oversized_files_and_expired_budgets() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    let alias = base.join("Alias.app");
    symlink(&root, &alias).unwrap();
    assert!(Inventory::inspect(&alias, deadline()).is_err());
    assert!(Inventory::inspect(&root, Instant::now()).is_err());
    let huge = root.join("Contents/huge");
    let file = std::fs::File::create(&huge).unwrap();
    file.set_len(2 * 1024 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(Inventory::inspect(&root, deadline()).is_err());
    std::fs::remove_file(huge).unwrap();
    let mut path = root.clone();
    for _ in 0..65 {
        path = path.join("deep");
        std::fs::create_dir(&path).unwrap();
    }
    assert!(Inventory::inspect(&root, deadline()).is_err());
}

#[test]
fn writable_source_files_are_reported_and_preserved_in_the_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let root = fixture(&temp.path().canonicalize().unwrap());
    let file = root.join("Contents/shader");
    std::fs::write(&file, b"synthetic shader").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o666)).unwrap();
    let source = Inventory::inspect(&root, deadline()).unwrap();
    assert_eq!(source.externally_writable_files(), 1);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let hardened = Inventory::inspect(&root, deadline()).unwrap();
    assert_eq!(hardened.externally_writable_files(), 0);
    assert_ne!(source.fingerprint(), hardened.fingerprint());
    assert!(source.revalidate(deadline()).is_err());
}

fn set_attribute(path: &Path, name: &str, value: &[u8]) {
    rustix::fs::setxattr(path, name, value, rustix::fs::XattrFlags::empty()).unwrap();
}
fn copy_fixture(source: &Path, copy: &Path) {
    let output = std::process::Command::new("/usr/bin/ditto")
        .args(["--rsrc", "--extattr", "--qtn", "--acl"])
        .arg(source)
        .arg(copy)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn resource_forks_quarantine_empty_values_and_directory_attributes_survive_copy() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let file = source.join("Contents/Frameworks/Thing.framework/Versions/A/Thing");
    set_attribute(&source, "org.qq-cli.synthetic", b"directory attribute");
    set_attribute(&file, "com.apple.ResourceFork", b"synthetic resource fork");
    set_attribute(&file, "com.apple.quarantine", b"0083;00000000;qq-cli-test;");
    set_attribute(&file, "org.qq-cli.empty", b"");
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let copy = base.join("Copy.app");
    copy_fixture(&source, &copy);
    assert_eq!(
        original
            .verify_copy(&copy, deadline())
            .unwrap()
            .fingerprint(),
        original.fingerprint()
    );
    original.revalidate(deadline()).unwrap();
}
#[test]
fn attribute_only_changes_in_copy_are_rejected_without_changing_data_fork() {
    for variant in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let source = fixture(&base);
        let relative = Path::new("Contents/Frameworks/Thing.framework/Versions/A/Thing");
        let file = source.join(relative);
        set_attribute(&file, "org.qq-cli.synthetic", b"before");
        set_attribute(&file, "com.apple.ResourceFork", b"resource");
        set_attribute(&file, "com.apple.quarantine", b"0083;00000000;qq-cli-test;");
        let original = Inventory::inspect(&source, deadline()).unwrap();
        let copy = base.join("Copy.app");
        copy_fixture(&source, &copy);
        original.verify_copy(&copy, deadline()).unwrap();
        let copied = copy.join(relative);
        let data = std::fs::read(&copied).unwrap();
        match variant {
            0 => set_attribute(&copied, "org.qq-cli.synthetic", b"after!"),
            1 => rustix::fs::removexattr(&copied, "com.apple.ResourceFork").unwrap(),
            2 => rustix::fs::removexattr(&copied, "com.apple.quarantine").unwrap(),
            _ => set_attribute(&copy, "org.qq-cli.extra", b"unexpected"),
        }
        assert_eq!(std::fs::read(&copied).unwrap(), data);
        assert!(original.verify_copy(&copy, deadline()).is_err());
        original.revalidate(deadline()).unwrap();
    }
}

#[test]
fn large_resource_fork_is_streamed_and_tail_tampering_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = fixture(&temp.path().canonicalize().unwrap());
    let file = root.join("Contents/large-fork");
    std::fs::write(&file, b"small data fork").unwrap();
    let mut value = vec![0_u8; 16 * 1024 * 1024 + 1];
    set_attribute(&file, "com.apple.ResourceFork", &value);
    let original = Inventory::inspect(&root, deadline()).unwrap();
    let copy = temp.path().canonicalize().unwrap().join("Copy.app");
    copy_fixture(&root, &copy);
    original.verify_copy(&copy, deadline()).unwrap();
    *value.last_mut().unwrap() = 1;
    set_attribute(
        &copy.join("Contents/large-fork"),
        "com.apple.ResourceFork",
        &value,
    );
    assert!(original.verify_copy(&copy, deadline()).is_err());
    original.revalidate(deadline()).unwrap();
}

fn metadata_command(tool: &str, args: &[&str], path: &Path) {
    let output = std::process::Command::new(tool)
        .args(args)
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn file_and_inheritable_directory_acls_survive_copy_but_changes_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let relative = Path::new("Contents/Frameworks/Thing.framework/Versions/A/Thing");
    metadata_command(
        "/bin/chmod",
        &["+a", "everyone allow read"],
        &source.join(relative),
    );
    metadata_command(
        "/bin/chmod",
        &["+a", "everyone allow list,file_inherit,directory_inherit"],
        &source,
    );
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let copy = base.join("Copy.app");
    copy_fixture(&source, &copy);
    original.verify_copy(&copy, deadline()).unwrap();
    metadata_command(
        "/bin/chmod",
        &["+a", "everyone deny write"],
        &copy.join(relative),
    );
    assert!(original.verify_copy(&copy, deadline()).is_err());
    original.revalidate(deadline()).unwrap();
    metadata_command("/bin/chmod", &["-N"], &source);
    assert!(original.revalidate(deadline()).is_err());
}

#[test]
fn filesystem_flags_are_preserved_and_flag_only_changes_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let relative = Path::new("Contents/Frameworks/Thing.framework/Versions/A/Thing");
    metadata_command("/usr/bin/chflags", &["hidden"], &source.join(relative));
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let copy = base.join("Copy.app");
    copy_fixture(&source, &copy);
    original.verify_copy(&copy, deadline()).unwrap();
    metadata_command("/usr/bin/chflags", &["nohidden"], &copy.join(relative));
    assert!(original.verify_copy(&copy, deadline()).is_err());
    original.revalidate(deadline()).unwrap();
    metadata_command("/usr/bin/chflags", &["nohidden"], &source.join(relative));
    assert!(original.revalidate(deadline()).is_err());
}

// Original 3000-byte "a" fixture encoded with zlib levels 1 and 9. Both decode
// identically, but their actual decmpfs metadata must remain distinguishable.
fn compressed_fixture(path: &Path, alternate: bool) {
    let compressed: &[u8] = if alternate {
        &[
            120, 218, 237, 193, 49, 1, 0, 0, 0, 194, 160, 172, 235, 95, 194, 26, 30, 64, 1, 0, 0,
            239, 6, 64, 172, 112, 245,
        ]
    } else {
        &[
            120, 1, 237, 208, 193, 0, 0, 0, 0, 3, 33, 214, 243, 151, 152, 198, 30, 133, 80, 97,
            192, 128, 1, 3, 6, 12, 24, 48, 96, 192, 192, 251, 192, 0, 64, 172, 112, 245,
        ]
    };
    std::fs::write(path, b"").unwrap();
    let mut value = Vec::new();
    value.extend_from_slice(&0x636d7066_u32.to_le_bytes());
    value.extend_from_slice(&3_u32.to_le_bytes());
    value.extend_from_slice(&3000_u64.to_le_bytes());
    value.extend_from_slice(compressed);
    set_attribute(path, "com.apple.decmpfs", &value);
    metadata_command("/usr/bin/chflags", &["40"], path); // Octal UF_COMPRESSED.
    assert_eq!(std::fs::read(path).unwrap(), vec![b'a'; 3000]);
}

#[test]
fn hidden_compression_is_inventoried_and_same_content_reencoding_is_rejected() {
    use std::os::macos::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let relative = Path::new("Contents/compressed");
    let file = source.join(relative);
    std::fs::write(&file, b"").unwrap();
    let before = Inventory::inspect(&source, deadline()).unwrap();
    compressed_fixture(&file, false);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    assert_eq!(
        original.extended_attribute_count(),
        before.extended_attribute_count() + 1
    );
    // Default xattr enumeration really hides the attribute in this fixture.
    let mut names = vec![0; 65536];
    let fd = std::fs::File::open(&file).unwrap();
    let size = rustix::fs::flistxattr(&fd, names.as_mut_slice()).unwrap();
    assert!(
        !names[..size]
            .split(|v| *v == 0)
            .any(|v| v == b"com.apple.decmpfs")
    );
    let copy = base.join("Copy.app");
    copy_fixture(&source, &copy);
    original.verify_copy(&copy, deadline()).unwrap();
    let copied = copy.join(relative);
    std::fs::remove_file(&copied).unwrap();
    compressed_fixture(&copied, true);
    assert_eq!(
        std::fs::read(&file).unwrap(),
        std::fs::read(&copied).unwrap()
    );
    assert_eq!(
        std::fs::metadata(&file).unwrap().st_flags(),
        std::fs::metadata(&copied).unwrap().st_flags()
    );
    assert!(original.verify_copy(&copy, deadline()).is_err());
    original.revalidate(deadline()).unwrap();
}

fn private_parent(base: &Path) -> std::path::PathBuf {
    use std::os::unix::fs::DirBuilderExt;
    let path = base.join("private");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .unwrap();
    path
}

#[test]
fn controlled_copy_preserves_complete_metadata_and_never_overwrites_a_target() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    compressed_fixture(&source.join("Contents/compressed"), false);
    set_attribute(&source, "org.qq-cli.directory", b"directory");
    set_attribute(
        &source,
        "com.apple.quarantine",
        b"0083;00000000;qq-cli-test;",
    );
    let writable = source.join("Contents/writable-source");
    std::fs::write(&writable, b"source mode must be preserved before hardening").unwrap();
    metadata_command("/bin/chmod", &["666"], &writable);
    let file = source.join("Contents/Frameworks/Thing.framework/Versions/A/Thing");
    set_attribute(&file, "com.apple.quarantine", b"0083;00000000;qq-cli-test;");
    metadata_command("/bin/chmod", &["+a", "everyone allow read"], &file);
    metadata_command("/usr/bin/chflags", &["hidden"], &file);
    metadata_command(
        "/bin/chmod",
        &["+a", "everyone allow list,file_inherit,directory_inherit"],
        &source,
    );
    metadata_command(
        "/bin/chmod",
        &["-h", "+a", "everyone deny write"],
        &source.join("Contents/Frameworks/Thing.framework/Thing"),
    );
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    let copied = original
        .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
        .unwrap();
    assert_eq!(copied.fingerprint(), original.fingerprint());
    assert_eq!(copied.externally_writable_files(), 1);
    copied.revalidate(deadline()).unwrap();
    original.revalidate(deadline()).unwrap();
    assert!(
        original
            .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
            .is_err()
    );
    copied.revalidate(deadline()).unwrap();
}

#[test]
fn copy_refuses_expired_changed_source_public_or_acl_granting_parents() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    assert!(
        original
            .copy_to_new(&parent, "QQ.app", Instant::now(), || Ok(()))
            .is_err()
    );
    metadata_command("/bin/chmod", &["755"], &parent);
    assert!(
        original
            .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
            .is_err()
    );
    metadata_command("/bin/chmod", &["700"], &parent);
    metadata_command("/bin/chmod", &["+a", "everyone allow list"], &parent);
    assert!(
        original
            .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
            .is_err()
    );
    metadata_command("/bin/chmod", &["-N"], &parent);
    std::fs::write(source.join("Contents/new"), b"changed").unwrap();
    assert!(
        original
            .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
            .is_err()
    );
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
}

#[test]
fn copy_guard_failure_retains_only_unaccepted_staging() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    let calls = std::cell::Cell::new(0);
    let result = original.copy_to_new(&parent, "QQ.app", deadline(), || {
        calls.set(calls.get() + 1);
        if calls.get() >= 3 {
            return Err(qq_cli::core::error::AppError::storage());
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(!parent.join("QQ.app").exists());
    let staging = std::fs::read_dir(&parent)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(staging.len(), 1);
    assert!(
        staging[0]
            .file_name()
            .to_str()
            .unwrap()
            .ends_with(".partial.app")
    );
    original.revalidate(deadline()).unwrap();
}

#[test]
fn copy_reports_failure_after_publication_without_deleting_the_copy() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    let result = original.copy_to_new(&parent, "QQ.app", deadline(), || {
        if parent.join("QQ.app").exists() {
            return Err(qq_cli::core::error::AppError::storage());
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(parent.join("QQ.app").exists());
    original
        .verify_copy(&parent.join("QQ.app"), deadline())
        .unwrap();
}

#[test]
fn parent_replacement_during_guard_never_publishes_to_the_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    let moved = base.join("moved");
    let replaced = std::cell::Cell::new(false);
    let result = original.copy_to_new(&parent, "QQ.app", deadline(), || {
        if !replaced.get() && std::fs::read_dir(&parent).unwrap().next().is_some() {
            std::fs::rename(&parent, &moved).unwrap();
            private_parent(&base);
            replaced.set(true);
        }
        Ok(())
    });
    assert!(result.is_err());
    assert!(replaced.get());
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    assert!(!moved.join("QQ.app").exists());
}

#[test]
fn hardening_requires_a_live_copy_capability_and_preserves_source_and_content() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let relative = Path::new("Contents/Frameworks/Thing.framework/Versions/A/Thing");
    let writable = source.join("Contents/writable");
    std::fs::write(&writable, b"private writable content").unwrap();
    metadata_command("/bin/chmod", &["666"], &writable);
    compressed_fixture(&source.join("Contents/compressed"), false);
    set_attribute(
        &source,
        "com.apple.quarantine",
        b"0083;00000000;qq-cli-test;",
    );
    metadata_command(
        "/bin/chmod",
        &["+a", "everyone allow list,file_inherit,directory_inherit"],
        &source,
    );
    metadata_command("/bin/chmod", &["+a", "everyone allow write"], &writable);
    metadata_command(
        "/bin/chmod",
        &["-h", "+a", "everyone allow read"],
        &source.join("Contents/Frameworks/Thing.framework/Thing"),
    );
    let original = Inventory::inspect(&source, deadline()).unwrap();
    // Scanning an original, even one owned by this UID, grants no write authority.
    assert!(
        Inventory::inspect(&source, deadline())
            .unwrap()
            .harden_copy(deadline(), || Ok(()))
            .is_err()
    );
    original.revalidate(deadline()).unwrap();
    let parent = private_parent(&base);
    let copy = original
        .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
        .unwrap();
    let root = parent.join("QQ.app");
    // Nor can inspecting an existing private copy recover that capability.
    assert!(
        Inventory::inspect(&root, deadline())
            .unwrap()
            .harden_copy(deadline(), || Ok(()))
            .is_err()
    );
    let old_fingerprint = copy.fingerprint().to_owned();
    let hardened = copy.harden_copy(deadline(), || Ok(())).unwrap();
    assert_ne!(hardened.fingerprint(), old_fingerprint);
    assert_eq!(hardened.externally_writable_files(), 0);
    assert_eq!(hardened.total_bytes(), original.total_bytes());
    assert_eq!(
        hardened.extended_attribute_count(),
        original.extended_attribute_count()
    );
    assert_eq!(std::fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
    assert_eq!(
        std::fs::metadata(root.join(relative)).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(root.join("Contents/writable"))
            .unwrap()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        std::fs::read(root.join("Contents/compressed")).unwrap(),
        vec![b'a'; 3000]
    );
    hardened.revalidate(deadline()).unwrap();
    original.revalidate(deadline()).unwrap();
    let fingerprint = hardened.fingerprint().to_owned();
    let repeated = hardened.harden_copy(deadline(), || Ok(())).unwrap();
    assert_eq!(repeated.fingerprint(), fingerprint);
}

#[test]
fn hardening_rejects_tampered_copy_and_guard_loss_without_touching_source() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source = fixture(&base);
    let original = Inventory::inspect(&source, deadline()).unwrap();
    let parent = private_parent(&base);
    let expired = original
        .copy_to_new(&parent, "Expired.app", deadline(), || Ok(()))
        .unwrap();
    let untouched = Inventory::inspect(&parent.join("Expired.app"), deadline()).unwrap();
    let error = expired
        .harden_copy(Instant::now(), || {
            panic!("expired work must not invoke guard")
        })
        .err()
        .unwrap();
    assert_eq!(error.code, "DEADLINE");
    untouched.revalidate(deadline()).unwrap();
    let copy = original
        .copy_to_new(&parent, "Changed.app", deadline(), || Ok(()))
        .unwrap();
    std::fs::write(parent.join("Changed.app/Contents/unexpected"), b"changed").unwrap();
    assert!(copy.harden_copy(deadline(), || Ok(())).is_err());
    let copy = original
        .copy_to_new(&parent, "Guard.app", deadline(), || Ok(()))
        .unwrap();
    let changed = std::cell::Cell::new(false);
    let executable = parent.join("Guard.app/Contents/Frameworks/Thing.framework/Versions/A/Thing");
    assert!(
        copy.harden_copy(deadline(), || {
            if std::fs::metadata(&executable).unwrap().permissions().mode() & 0o777 == 0o700 {
                changed.set(true);
                return Err(qq_cli::core::error::AppError::storage());
            }
            Ok(())
        })
        .is_err()
    );
    assert!(changed.get());
    assert!(parent.join("Guard.app").exists());
    original.revalidate(deadline()).unwrap();
}

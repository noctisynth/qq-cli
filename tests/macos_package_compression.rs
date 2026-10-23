#![cfg(all(target_os = "macos", target_arch = "aarch64"))]

use qq_cli::core::error::AppError;
use qq_cli::napcat::install::macos_bundle::Inventory;
use qq_cli::napcat::runtime::macos_adapter::Prepared;
use std::os::macos::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

const PACKAGE: &str = "Contents/Resources/app/package.json";
const SOURCE: &[u8] = br#"{"name":"qq-chat","platform":"darwin","version":"synthetic","buildVersion":"1","main":"./application.asar/app_launcher/index.js"}"#;
// zlib encoding of the original synthetic SOURCE above, not an upstream asset.
const ZLIB: &[u8] = &[
    120, 156, 61, 201, 49, 14, 194, 48, 12, 70, 225, 187, 120, 134, 84, 172, 28, 132, 21, 253, 36,
    70, 49, 74, 220, 212, 73, 129, 10, 113, 119, 220, 133, 237, 233, 123, 31, 82, 84, 166, 51, 45,
    203, 49, 102, 12, 58, 80, 43, 24, 247, 217, 170, 99, 130, 189, 68, 221, 158, 108, 93, 102, 117,
    234, 155, 142, 204, 67, 162, 235, 109, 149, 146, 46, 255, 117, 114, 170, 144, 61, 195, 132,
    214, 138, 68, 12, 95, 1, 29, 182, 195, 181, 96, 213, 152, 217, 38, 209, 196, 239, 240, 232,
    244, 253, 1, 20, 153, 44, 147,
];

fn attribute(path: &Path, name: &str, bytes: &[u8]) {
    rustix::fs::setxattr(path, name, bytes, rustix::fs::XattrFlags::empty()).unwrap();
}

fn compress(path: &Path, resource: bool, independent_fork: bool) {
    std::fs::write(path, b"").unwrap();
    let mut header = 0x636d7066_u32.to_le_bytes().to_vec();
    header.extend_from_slice(&(if resource { 4_u32 } else { 3_u32 }).to_le_bytes());
    header.extend_from_slice(&(SOURCE.len() as u64).to_le_bytes());
    if resource {
        // One zlib block in a classic resource fork: big-endian outer header,
        // little-endian block table, and a single cmpf resource map entry.
        let end = 272 + ZLIB.len() as u32;
        let mut fork = vec![0; 272];
        for (offset, value) in [
            (0, 256_u32),
            (4, end),
            (8, end - 256),
            (12, 50),
            (256, end - 260),
        ] {
            fork[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        for (offset, value) in [(260, 1_u32), (264, 12), (268, ZLIB.len() as u32)] {
            fork[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        fork.extend_from_slice(ZLIB);
        let mut map = vec![0; 50];
        for (offset, value) in [(24, 28_u16), (26, 50), (36, 10), (38, 1), (40, u16::MAX)] {
            map[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
        }
        map[30..34].copy_from_slice(b"cmpf");
        fork.extend_from_slice(&map);
        attribute(path, "com.apple.ResourceFork", &fork);
    } else {
        header.extend_from_slice(ZLIB);
        if independent_fork {
            attribute(path, "com.apple.ResourceFork", b"independent resource");
        }
    }
    attribute(path, "com.apple.decmpfs", &header);
    attribute(path, "com.apple.quarantine", b"0081;00000000;Synthetic;");
    attribute(path, "org.qq-cli.keep", b"synthetic attribute");
    assert!(
        std::process::Command::new("/usr/bin/chflags")
            .arg("40")
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(std::fs::read(path).unwrap(), SOURCE);
    assert_ne!(
        std::fs::metadata(path).unwrap().st_flags() & libc::UF_COMPRESSED,
        0
    );
}

#[test]
fn compressed_entry_normalizes_before_edit_and_preserves_noncompression_metadata() {
    use sha2::{Digest, Sha256};
    for variant in 0..7 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let source = base.join("Source.app");
        std::fs::create_dir_all(source.join("Contents/Resources/app")).unwrap();
        compress(&source.join(PACKAGE), variant == 2, variant == 1);
        let root = base.join("instance");
        let framework = base.join("framework");
        std::fs::create_dir(&framework).unwrap();
        let home = root.join("profile-home/Library/Containers/com.tencent.qq/Data");
        for leaf in [
            "Desktop",
            "Documents",
            "Downloads",
            "Music",
            "Pictures",
            "Movies",
            "Library/Application Support/QQ",
        ] {
            std::fs::create_dir_all(home.join(leaf)).unwrap();
        }
        for leaf in ["temp", "logs", "config", "cache", "napcat-data", "adapter"] {
            std::fs::create_dir_all(root.join(leaf)).unwrap();
        }
        for path in [&root, &root.join("adapter")] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let plan = Prepared::inspect(&root, &framework).unwrap();
        plan.publish_adapter(|| Ok(())).unwrap();
        let edit = plan
            .package_edit(
                SOURCE,
                &format!("{:x}", Sha256::digest(SOURCE)),
                "synthetic",
                "1",
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let original = Inventory::inspect(&source, deadline).unwrap();
        let copy = original
            .copy_to_new(&root, "QQ.app", deadline, || Ok(()))
            .unwrap()
            .harden_copy(deadline, || Ok(()))
            .unwrap();
        let calls = std::cell::Cell::new(0);
        let outcome = plan.deploy_package(copy, &edit, deadline, || {
            calls.set(calls.get() + 1);
            if (variant == 3 && calls.get() == 2) || (variant == 4 && calls.get() == 3) {
                return Err(AppError::storage());
            }
            if calls.get() == 3 && variant == 5 {
                attribute(edit.destination(), "com.apple.quarantine", b"changed");
            }
            if calls.get() == 3 && variant == 6 {
                std::fs::write(root.join("QQ.app/extra"), b"changed").unwrap();
            }
            Ok(())
        });
        original.revalidate(deadline).unwrap();
        if variant < 3 {
            outcome.unwrap().revalidate(deadline).unwrap();
            assert_eq!(std::fs::read(edit.destination()).unwrap(), edit.bytes());
            let mut value = [0; 128];
            for (name, expected) in [
                (
                    "com.apple.quarantine",
                    b"0081;00000000;Synthetic;".as_slice(),
                ),
                ("org.qq-cli.keep", b"synthetic attribute"),
            ] {
                let n = rustix::fs::getxattr(edit.destination(), name, &mut value).unwrap();
                assert_eq!(&value[..n], expected);
            }
            if variant == 1 {
                let n =
                    rustix::fs::getxattr(edit.destination(), "com.apple.ResourceFork", &mut value)
                        .unwrap();
                assert_eq!(&value[..n], b"independent resource");
            }
            if variant == 2 {
                assert!(
                    rustix::fs::getxattr(edit.destination(), "com.apple.ResourceFork", &mut value)
                        .is_err()
                );
            }
        } else {
            assert!(outcome.is_err(), "variant {variant}");
            assert_eq!(std::fs::read(edit.destination()).unwrap(), SOURCE);
        }
        assert_eq!(
            std::fs::metadata(edit.destination()).unwrap().st_flags() & libc::UF_COMPRESSED,
            if variant == 3 { libc::UF_COMPRESSED } else { 0 }
        );
    }
}

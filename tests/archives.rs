use std::io::{Cursor, Write};
use std::time::{Duration, Instant};

use qq_cli::napcat::install::archive::{self, Expected};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in entries {
        writer
            .start_file(
                *path,
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn verified_archive_has_an_owned_manifest_and_failure_leaves_no_partial_tree() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = zip(&[
        ("loader.exe", b"synthetic loader bytes"),
        ("native/module.node", b"synthetic native bytes"),
    ]);
    let hash = digest(&bytes);
    let loader = digest(b"synthetic loader bytes");
    let expected = Expected {
        bytes: bytes.len() as u64,
        sha256: &hash,
        files: 2,
        required: &[("loader.exe", &loader)],
    };
    let prepared =
        archive::prepare(Cursor::new(&bytes), &expected, temp.path(), deadline()).unwrap();
    assert_eq!(prepared.members.len(), 2);
    assert_eq!(prepared.members[0].sha256, loader);
    assert_eq!(
        std::fs::read(prepared.directory.path().join("native/module.node")).unwrap(),
        b"synthetic native bytes"
    );
    drop(prepared);
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    let bad = Expected {
        sha256: &"0".repeat(64),
        ..expected
    };
    assert_eq!(
        archive::prepare(Cursor::new(&bytes), &bad, temp.path(), deadline())
            .err()
            .unwrap()
            .code,
        "ARTIFACT_MISMATCH"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn traversal_windows_aliases_case_collisions_and_symlinks_are_rejected() {
    for name in [
        "../outside",
        "/absolute",
        "C:relative",
        "a\\b",
        "a/../b",
        "a//b",
        "a/./b",
        "a:stream",
        "aux.txt",
        "COM1.node",
        "a. ",
        ".qq-cli-installation.json",
    ] {
        assert!(archive::member_path(name).is_err(), "{name}");
    }
    let temp = tempfile::tempdir().unwrap();
    let mut cases = vec![
        zip(&[("Name.txt", b"one"), ("name.txt", b"two")]),
        zip(&[("Native/one.node", b"one"), ("native/two.node", b"two")]),
        zip(&[("../outside.txt", b"synthetic")]),
    ];
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink("link", "../outside", SimpleFileOptions::default())
        .unwrap();
    cases.push(writer.finish().unwrap().into_inner());
    for bytes in cases {
        let hash = digest(&bytes);
        let expected = Expected {
            bytes: bytes.len() as u64,
            sha256: &hash,
            files: 1,
            required: &[],
        };
        assert_eq!(
            archive::prepare(Cursor::new(bytes), &expected, temp.path(), deadline())
                .err()
                .unwrap()
                .code,
            "ARCHIVE_UNSAFE"
        );
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }
}

#[test]
fn missing_pinned_members_file_count_deadline_and_resource_limits_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let bytes = zip(&[("file.txt", b"sample")]);
    let hash = digest(&bytes);
    let expected = Expected {
        bytes: bytes.len() as u64,
        sha256: &hash,
        files: 1,
        required: &[("missing.exe", &hash)],
    };
    assert_eq!(
        archive::prepare(Cursor::new(&bytes), &expected, temp.path(), deadline())
            .err()
            .unwrap()
            .code,
        "ARTIFACT_INCOMPLETE"
    );
    let expected = Expected {
        required: &[],
        ..expected
    };
    assert_eq!(
        archive::prepare(Cursor::new(&bytes), &expected, temp.path(), Instant::now())
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    let huge = Expected {
        bytes: 65 * 1024 * 1024,
        ..expected
    };
    assert_eq!(
        archive::prepare(Cursor::new(&bytes), &huge, temp.path(), deadline())
            .err()
            .unwrap()
            .code,
        "ARCHIVE_UNSAFE"
    );
    let wrong_count = Expected {
        files: 2,
        ..expected
    };
    assert_eq!(
        archive::prepare(Cursor::new(&bytes), &wrong_count, temp.path(), deadline())
            .err()
            .unwrap()
            .code,
        "ARCHIVE_UNSAFE"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    let position = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    for (offset, value) in [(16, 0_u32), (24, 129 * 1024 * 1024)] {
        let mut invalid = bytes.clone();
        invalid[position + offset..position + offset + 4].copy_from_slice(&value.to_le_bytes());
        let hash = digest(&invalid);
        let expected = Expected {
            bytes: invalid.len() as u64,
            sha256: &hash,
            files: 1,
            required: &[],
        };
        assert_eq!(
            archive::prepare(Cursor::new(invalid), &expected, temp.path(), deadline())
                .err()
                .unwrap()
                .code,
            "ARCHIVE_UNSAFE"
        );
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }
}

#[test]
fn catalog_is_exact_and_records_unresolved_authorization_without_treating_download_as_permission() {
    for version in ["latest", "4.18.39", "vv4.18.38", ""] {
        assert!(qq_cli::napcat::install::catalog::lookup(version).is_err());
    }
    for version in [
        "v4.18.41", "4.18.41", "v4.18.38", "4.18.38", "v4.18.37", "4.18.37",
    ] {
        let release = qq_cli::napcat::install::catalog::lookup(version).unwrap();
        assert_eq!(release.archive.files, 680);
        assert_eq!(
            release.archive.bytes,
            match release.version {
                "v4.18.41" => 36_738_019,
                "v4.18.38" => 36_728_573,
                "v4.18.37" => 36_723_625,
                _ => unreachable!(),
            }
        );
        assert!(release.authorization_evidence.is_none());
        assert!(
            release
                .archive
                .required
                .iter()
                .all(|(_, hash)| hash.len() == 64)
        );
    }
    let current = qq_cli::napcat::install::catalog::lookup("4.18.38").unwrap();
    let previous = qq_cli::napcat::install::catalog::lookup("4.18.37").unwrap();
    assert_ne!(current.commit, previous.commit);
    assert_ne!(current.archive.sha256, previous.archive.sha256);
    let payload = |release: &qq_cli::napcat::install::catalog::Release| {
        release
            .archive
            .required
            .iter()
            .find(|(name, _)| *name == "napcat.mjs")
            .unwrap()
            .1
    };
    assert_ne!(payload(&current), payload(&previous));
}

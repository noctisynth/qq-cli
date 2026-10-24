use super::*;
use crate::napcat::install::archive::Expected;
use crate::storage::state::private_dir;
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Write},
    time::Duration,
};

#[test]
fn completed_deployments_still_require_authenticated_framework_and_exact_adapter() {
    for variant in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let store = Store::new(base.join("state"));
        let refs = References::with_key(store.root().join("references"), [23; 32]);
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("payload", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"original synthetic framework").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let release = catalog::Release {
            version: "fixture",
            commit: "synthetic",
            url: "https://example.invalid/synthetic",
            archive: Expected {
                bytes: bytes.len() as u64,
                sha256: Box::leak(digest.into_boxed_str()),
                files: 1,
                required: &[],
            },
            authorization_evidence: None,
        };
        installation::import(Cursor::new(bytes), &release, &store, &refs, deadline).unwrap();
        let installed = store.transaction(deadline).unwrap().registry.installations[0].clone();
        let root = base.join("runtime");
        for leaf in [
            "Desktop",
            "Documents",
            "Downloads",
            "Music",
            "Pictures",
            "Movies",
            "Library/Application Support/QQ",
        ] {
            private_dir(
                &root
                    .join("profile-home/Library/Containers/com.tencent.qq/Data")
                    .join(leaf),
            )
            .unwrap();
        }
        for leaf in [
            "temp",
            "logs",
            "config",
            "cache",
            "napcat-data/config",
            "napcat-data/logs",
            "napcat-data/cache",
            "napcat-data/plugins",
            "adapter",
        ] {
            private_dir(&root.join(leaf)).unwrap();
        }
        let prepared = Prepared::inspect(&root, &installed.directory).unwrap();
        let adapter = prepared.adapter_plan().unwrap();
        assert!(verify_published(&store, &refs, &installed, &adapter, deadline).is_err());
        adapter.publish(|| Ok(())).unwrap();
        verify_published(&store, &refs, &installed, &adapter, deadline).unwrap();
        match variant {
            0 => std::fs::write(installed.directory.join("payload"), b"changed").unwrap(),
            1 => std::fs::write(installed.directory.join("extra"), b"unknown").unwrap(),
            2 => std::fs::write(adapter.entry(), b"changed").unwrap(),
            _ => {}
        }
        let wrong = References::with_key(store.root().join("references"), [24; 32]);
        assert!(
            verify_published(
                &store,
                if variant == 3 { &wrong } else { &refs },
                &installed,
                &adapter,
                deadline
            )
            .is_err()
        );
    }
}

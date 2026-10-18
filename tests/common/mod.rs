use qq_cli::napcat::install::archive::Expected;
use qq_cli::napcat::install::catalog::Release;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

pub fn synthetic_release() -> (Vec<u8>, Release) {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, body) in [
        ("loader.exe", b"synthetic loader".as_slice()),
        ("native/module.node", b"synthetic native".as_slice()),
    ] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(body).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let digest = Box::leak(format!("{:x}", Sha256::digest(&bytes)).into_boxed_str());
    let release = Release {
        version: "v0.0.0",
        commit: "synthetic-owned-source",
        url: "https://invalid.example/synthetic-owned.zip",
        archive: Expected {
            bytes: bytes.len() as u64,
            sha256: digest,
            files: 2,
            required: &[],
        },
        authorization_evidence: Some("original synthetic fixture"),
    };
    (bytes, release)
}

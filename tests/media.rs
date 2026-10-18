use std::time::{Duration, Instant};

use qq_cli::messaging::media::{MAX_DOWNLOAD_BYTES, copy_from_cache};
use sha2::{Digest, Sha256};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn cache_transfer_is_atomic_hashed_and_never_overwrites() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let source = cache.join("image.bin");
    let bytes = b"original synthetic media bytes";
    std::fs::write(&source, bytes).unwrap();
    let output = temp.path().join("download.bin");
    let receipt = copy_from_cache(
        &source,
        std::slice::from_ref(&cache),
        &output,
        Some(bytes.len() as u64),
        deadline(),
    )
    .unwrap();
    assert_eq!(receipt.size_bytes, bytes.len() as u64);
    assert_eq!(receipt.sha256, format!("{:x}", Sha256::digest(bytes)));
    assert_eq!(std::fs::read(&output).unwrap(), bytes);
    let error = copy_from_cache(&source, &[cache], &output, None, deadline()).unwrap_err();
    assert_eq!(error.code, "MEDIA_OUTPUT_INVALID");
    assert_eq!(std::fs::read(output).unwrap(), bytes);
}

#[test]
fn untrusted_outside_oversized_and_changed_sources_never_publish_output() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let source = cache.join("file.bin");
    std::fs::write(&source, b"fixture").unwrap();
    let outside = temp.path().join("outside.bin");
    std::fs::write(&outside, b"outside").unwrap();
    let output = temp.path().join("result.bin");
    for candidate in [&outside, &cache.join("..").join("outside.bin"), &cache] {
        assert_eq!(
            copy_from_cache(
                candidate,
                std::slice::from_ref(&cache),
                &output,
                None,
                deadline()
            )
            .unwrap_err()
            .code,
            "MEDIA_SOURCE_INVALID"
        );
        assert!(!output.exists());
    }
    assert_eq!(
        copy_from_cache(&source, &[], &output, None, deadline())
            .unwrap_err()
            .code,
        "MEDIA_SOURCE_INVALID"
    );
    assert_eq!(
        copy_from_cache(
            &source,
            std::slice::from_ref(&cache),
            &output,
            Some(8),
            deadline()
        )
        .unwrap_err()
        .code,
        "MEDIA_CHANGED"
    );
    std::fs::OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_len(MAX_DOWNLOAD_BYTES + 1)
        .unwrap();
    assert_eq!(
        copy_from_cache(&source, &[cache], &output, None, deadline())
            .unwrap_err()
            .code,
        "MEDIA_SOURCE_INVALID"
    );
    assert!(!output.exists());
}

#[test]
fn unsafe_output_names_and_expired_deadlines_leave_no_temporary_files() {
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("cache");
    std::fs::create_dir(&cache).unwrap();
    let source = cache.join("file.bin");
    std::fs::write(&source, b"fixture").unwrap();
    for name in ["CON", "file:stream", "trailing.", "missing/output.bin"] {
        assert_eq!(
            copy_from_cache(
                &source,
                std::slice::from_ref(&cache),
                &temp.path().join(name),
                None,
                deadline()
            )
            .unwrap_err()
            .code,
            "MEDIA_OUTPUT_INVALID"
        );
    }
    assert_eq!(
        copy_from_cache(
            &source,
            &[cache],
            &temp.path().join("output.bin"),
            None,
            Instant::now()
        )
        .unwrap_err()
        .code,
        "DEADLINE"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn backend_base64_is_hashed_and_published_only_after_identity_guard() {
    use base64::Engine;
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("decoded.bin");
    let bytes = b"original synthetic backend bytes";
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let guarded = std::cell::Cell::new(false);
    let transfer = qq_cli::messaging::media::write_base64(
        &encoded,
        &output,
        Some(bytes.len() as u64),
        deadline(),
        || {
            guarded.set(true);
            Ok(())
        },
    )
    .unwrap();
    assert!(guarded.get());
    assert_eq!(std::fs::read(&output).unwrap(), bytes);
    assert_eq!(transfer.sha256, format!("{:x}", Sha256::digest(bytes)));
    assert_eq!(
        qq_cli::messaging::media::write_base64(&encoded, &output, None, deadline(), || Ok(()))
            .unwrap_err()
            .code,
        "MEDIA_OUTPUT_INVALID"
    );
}

#[test]
fn malformed_or_mismatched_backend_bytes_leave_no_partial_output() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("decoded.bin");
    for invalid in ["%%%", "YQ", "YQ==extra", "https://invalid.example/"] {
        assert_eq!(
            qq_cli::messaging::media::write_base64(invalid, &output, None, deadline(), || Ok(()))
                .unwrap_err()
                .code,
            "MEDIA_BYTES_INVALID"
        );
        assert!(!output.exists());
    }
    assert_eq!(
        qq_cli::messaging::media::write_base64("YQ==", &output, Some(2), deadline(), || Ok(()))
            .unwrap_err()
            .code,
        "MEDIA_CHANGED"
    );
    assert_eq!(
        qq_cli::messaging::media::write_base64("YQ==", &output, None, deadline(), || Err(
            qq_cli::core::error::AppError::new("ACCOUNT_CHANGED", "Synthetic identity change", 4)
        ))
        .unwrap_err()
        .code,
        "ACCOUNT_CHANGED"
    );
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

use qq_cli::{napcat::install::acquire, storage::references::References, storage::state::Store};
use serde_json::Value;
use std::time::{Duration, Instant};
mod common;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn local_source_is_verified_and_cached_import_does_not_reopen_it() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [53; 32]);
    let (bytes, release) = common::synthetic_release();
    let path = temp.path().join("original.zip");
    std::fs::write(&path, bytes).unwrap();
    acquire::install(&release, Some(&path), &store, &refs, deadline()).unwrap();
    std::fs::remove_file(&path).unwrap();
    let cached: Value = serde_json::to_value(
        acquire::install(&release, Some(&path), &store, &refs, deadline()).unwrap(),
    )
    .unwrap();
    assert_eq!(cached["data"]["cached"], true);
    assert_eq!(store.read().unwrap().installations.len(), 1);
}

#[test]
fn rejected_sources_create_no_installation_and_do_not_echo_paths() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [54; 32]);
    let (_, release) = common::synthetic_release();
    let path = temp.path().join("synthetic-private-input.zip");
    let error = acquire::install(&release, Some(&path), &store, &refs, deadline())
        .err()
        .unwrap();
    assert_eq!(error.code, "ARTIFACT_SOURCE_UNAVAILABLE");
    assert!(
        !serde_json::to_string(&error)
            .unwrap()
            .contains("synthetic-private-input")
    );
    assert!(store.read().unwrap().installations.is_empty());
    std::fs::write(&path, b"original wrong size").unwrap();
    assert_eq!(
        acquire::install(&release, Some(&path), &store, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "ARTIFACT_SOURCE_UNAVAILABLE"
    );
    let (mut bytes, _) = common::synthetic_release();
    bytes[0] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        acquire::install(&release, Some(&path), &store, &refs, deadline())
            .err()
            .unwrap()
            .code,
        "ARTIFACT_MISMATCH"
    );
    assert!(store.read().unwrap().installations.is_empty());
}

#[test]
fn public_install_checks_catalog_before_platform_credentials_or_local_source() {
    use std::process::Command;
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let config = temp.path().join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&qq_cli::storage::config::Config {
            state_dir: Some(state.clone()),
            ..Default::default()
        })
        .unwrap(),
    )
    .unwrap();
    let missing = temp.path().join("synthetic-private-input.zip");
    // Import cannot inspect the source until the original signing authority is available.
    // Keep this explicit until a protected vault is implemented on the host platform.
    let (source_code, source_exit) = if cfg!(windows) {
        ("ARTIFACT_SOURCE_UNAVAILABLE", 6)
    } else {
        ("PLATFORM_UNSUPPORTED", 4)
    };
    for (version, code, exit) in [
        ("latest", "VERSION_UNSUPPORTED", 4),
        ("v4.18.38", source_code, source_exit),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(["napcat", "install", "--version", version, "--from"])
            .arg(&missing)
            .arg("--config")
            .arg(&config)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(exit));
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], code);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("synthetic-private-input"));
        assert!(output.stderr.is_empty());
        if version == "latest" {
            assert!(!state.exists());
        }
    }
}

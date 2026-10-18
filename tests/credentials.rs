#[cfg(windows)]
#[test]
fn system_vault_round_trip_is_isolated_to_a_random_temporary_state_namespace() {
    use qq_cli::credentials::{Credentials, SystemCredentials};
    struct Cleanup(SystemCredentials);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete("synthetic-selftest");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let credentials = Cleanup(SystemCredentials::new(temp.path()).unwrap());
    assert_eq!(
        credentials.0.get("synthetic-selftest").unwrap_err().code,
        "CREDENTIAL_NOT_FOUND"
    );
    credentials
        .0
        .set("synthetic-selftest", "qq-cli-synthetic-test-value")
        .unwrap();
    assert_eq!(
        credentials.0.get("synthetic-selftest").unwrap(),
        "qq-cli-synthetic-test-value"
    );
    credentials.0.delete("synthetic-selftest").unwrap();
    assert_eq!(
        credentials.0.get("synthetic-selftest").unwrap_err().code,
        "CREDENTIAL_NOT_FOUND"
    );
    credentials.0.delete("synthetic-selftest").unwrap();
}
#[cfg(windows)]
mod common;

#[cfg(windows)]
#[test]
fn cli_uninstall_uses_only_its_temporary_vault_key_and_signed_synthetic_installation() {
    use qq_cli::credentials::{Credentials, SystemCredentials};
    use qq_cli::references::References;
    use qq_cli::state::Store;
    use std::time::{Duration, Instant};
    struct Cleanup(SystemCredentials);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let deadline = || Instant::now() + Duration::from_secs(5);
    let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
    let references = References::open(&store, &credentials.0, deadline()).unwrap();
    let (bytes, release) = common::synthetic_release();
    qq_cli::installation::import(
        std::io::Cursor::new(bytes),
        &release,
        &store,
        &references,
        deadline(),
    )
    .unwrap();
    let installed = store.read().unwrap().installations[0].clone();
    let config = temp.path().join("config.json");
    std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(["napcat", "uninstall", &installed.id, "--config"])
        .arg(config)
        .env_remove("QQ_CLI_CONFIG")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["removed"], true);
    assert!(!installed.directory.exists());
    assert!(store.read().unwrap().installations.is_empty());
    credentials.0.delete("reference-signing-key").unwrap();
}

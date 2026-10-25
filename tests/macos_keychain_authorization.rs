#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use clap::Parser;
use qq_cli::cli::args::{Cli, Command, NapcatCommand};
use std::process::Command as Process;

#[test]
fn keychain_authorization_is_explicit_scoped_and_does_not_imply_a_startup_trial() {
    let parse = |extra: &[&str]| {
        let mut args = vec!["qq-cli", "napcat", "start"];
        args.extend_from_slice(extra);
        Cli::try_parse_from(args)
    };
    for (extra, expected) in [(vec![], false), (vec!["--authorize-keychain"], true)] {
        let Command::Napcat {
            command: NapcatCommand::Start(args),
        } = parse(&extra).unwrap().command
        else {
            panic!("wrong command");
        };
        assert_eq!(args.authorize_keychain, expected);
        assert!(!args.new && !args.verify_compatibility);
    }
    assert!(parse(&["--authorize-keychain", "--verify-compatibility"]).is_err());
    assert!(Cli::try_parse_from(["qq-cli", "doctor", "--authorize-keychain"]).is_err());
    let schema = qq_cli::cli::registry::get("napcat.start").unwrap();
    assert_eq!(
        schema["inputSchema"]["properties"]["authorize_keychain"]["type"],
        "boolean"
    );
}

#[test]
fn malformed_authorization_workers_are_silent_and_refuse_before_vault_access() {
    for args in [
        vec![],
        vec!["other-service", "reference-signing-key", "1000"],
        vec!["qq-cli-short", "reference-signing-key", "1000"],
        vec![
            "qq-cli-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "arbitrary-item",
            "1000",
        ],
        vec![
            "qq-cli-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "reference-signing-key",
            "0",
        ],
    ] {
        let output = Process::new(env!("CARGO_BIN_EXE_qq-cli"))
            .arg("--internal-macos-keychain-authorization-v1")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(93));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
}

#[test]
fn missing_installation_refuses_before_authorization_or_state_creation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let config = root.join("config.json");
    std::fs::write(&config, b"{}").unwrap();
    let output = Process::new(env!("CARGO_BIN_EXE_qq-cli"))
        .arg("--config")
        .arg(config)
        .args(["napcat", "start", "--authorize-keychain"])
        .output()
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "INSTALLATION_NOT_FOUND");
    assert!(!root.join("state").exists());
}

#[test]
fn a_foreign_credential_key_is_rejected_before_the_system_authorization_dialog() {
    use qq_cli::storage::state::{Installation, Instance, Store};
    use std::time::{Duration, Instant};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let config = root.join("config.json");
    std::fs::write(&config, b"{}").unwrap();
    let store = Store::new(root.join("state"));
    let mut tx = store
        .transaction(Instant::now() + Duration::from_secs(5))
        .unwrap();
    tx.registry.installations.push(Installation {
        id: "install-example".into(),
        version: "v4.18.41".into(),
        digest: "a".repeat(64),
        directory: root.join("not-an-installed-artifact"),
    });
    tx.registry.instances.push(Instance {
        id: "instance-example".into(),
        run_id: "run-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "foreign-key-must-not-be-read".into(),
        account: None,
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let original = store.read().unwrap().instances;
    let output = Process::new(env!("CARGO_BIN_EXE_qq-cli"))
        .arg("--config")
        .arg(config)
        .args([
            "--instance",
            "instance-example",
            "napcat",
            "start",
            "--authorize-keychain",
        ])
        .output()
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "START_INPUT_CHANGED");
    assert_eq!(store.read().unwrap().instances, original);
}

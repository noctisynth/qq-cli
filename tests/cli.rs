use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use clap::{CommandFactory, Parser};
use qq_cli::cli::args::Cli;
use serde_json::Value;

fn run(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .env("QQ_CLI_TOKEN", "synthetic-secret-never-echo")
        .env_remove("QQ_CLI_CONFIG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("CLI unexpectedly waited for interactive input");
        }
        thread::sleep(Duration::from_millis(5));
    }
    drop(input);
    child.wait_with_output().unwrap()
}

fn body(output: &Output) -> Value {
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(text.lines().count(), 1);
    assert!(!text.contains("synthetic-secret-never-echo"));
    serde_json::from_str(text).unwrap()
}

#[test]
fn version_trial_is_explicit_and_uses_the_total_deadline() {
    let rollback =
        Cli::try_parse_from(["qq-cli", "napcat", "rollback", "--timeout", "120"]).unwrap();
    assert_eq!(rollback.command.timeout_seconds(), 120);
    for extra in [vec![], vec!["--verify-compatibility"]] {
        let mut args = vec![
            "qq-cli",
            "napcat",
            "update",
            "--version",
            "v4.18.37",
            "--timeout",
            "120",
        ];
        args.extend(extra.clone());
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.command.timeout_seconds(), 120);
        let qq_cli::cli::args::Command::Napcat {
            command:
                qq_cli::cli::args::NapcatCommand::Update {
                    verify_compatibility,
                    ..
                },
        } = cli.command
        else {
            panic!("wrong command")
        };
        assert_eq!(verify_compatibility, !extra.is_empty());
    }
    let schema = qq_cli::cli::registry::get("napcat.update").unwrap();
    assert_eq!(
        schema["inputSchema"]["properties"]["verify_compatibility"]["type"],
        "boolean"
    );
}

#[test]
fn startup_trial_requires_explicit_new_instance_and_stable_operation_key() {
    assert!(Cli::try_parse_from(["qq-cli", "napcat", "start", "--verify-compatibility"]).is_err());
    assert!(
        Cli::try_parse_from([
            "qq-cli",
            "napcat",
            "start",
            "--verify-compatibility",
            "--new"
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            "qq-cli",
            "napcat",
            "start",
            "--verify-compatibility",
            "--new",
            "--request-id",
            "synthetic-trial"
        ])
        .is_ok()
    );
}

#[test]
fn explicit_config_and_request_lookup_do_not_need_default_home_or_an_instance() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config.json");
    std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
    for command in [
        vec!["status"],
        vec!["request", "get", "synthetic-key"],
        vec!["napcat", "uninstall", "synthetic-missing-installation"],
        vec!["contact", "list"],
        vec!["napcat", "stop"],
        vec!["napcat", "remove", "missing-instance"],
        vec!["message", "history", "--conversation", "group:10001"],
        vec!["doctor", "--version", "latest"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(command)
            .arg("--config")
            .arg(&config)
            .env_remove("LOCALAPPDATA")
            .env_remove("HOME")
            .env_remove("XDG_DATA_HOME")
            .env_remove("QQ_CLI_CONFIG")
            .output()
            .unwrap();
        let value = body(&output);
        if value["data"]["state"] == "not_started" {
            let descriptor = qq_cli::cli::registry::get("status").unwrap();
            assert!(
                jsonschema::validator_for(&descriptor["responseSchema"])
                    .unwrap()
                    .is_valid(&value)
            );
            assert_eq!(value["data"]["identityVerified"], false);
        }
        assert!(
            value["data"]["state"] == "not_started"
                || value["error"]["code"] == "REQUEST_NOT_FOUND"
                || value["error"]["code"] == "INSTALLATION_NOT_FOUND"
                || value["error"]["code"] == "INSTANCE_NOT_FOUND"
                || value["error"]["code"] == "VERSION_UNSUPPORTED"
        );
        assert!(!temp.path().join("state").exists());
    }
}

#[test]
fn registry_and_every_leaf_help_use_the_same_command_tree_offline() {
    Cli::command().debug_assert();
    let output = run(&["schema", "list", "--config", "missing-config.json"]);
    assert!(output.status.success());
    let result = body(&output);
    let commands = result["data"].as_array().unwrap();
    assert_eq!(commands.len(), 34);
    for command in commands {
        assert!(!command["summary"].as_str().unwrap().is_empty());
        let name = command["name"].as_str().unwrap();
        let mut args = name.split('.').collect::<Vec<_>>();
        args.push("--help");
        let help = run(&args);
        assert!(help.status.success(), "{name}");
        assert!(!help.stdout.is_empty());
        assert!(help.stderr.is_empty());
    }
    assert!(run(&["--version"]).status.success());
}

#[test]
fn schema_exposes_real_flags_defaults_and_bounds_without_config_io() {
    let output = run(&[
        "schema",
        "get",
        "message.history",
        "--config",
        "missing-config.json",
    ]);
    assert!(output.status.success());
    let value = body(&output);
    assert_eq!(
        value["data"]["inputSchema"]["properties"]["limit"]["default"],
        20
    );
    assert_eq!(
        value["data"]["inputSchema"]["properties"]["limit"]["maximum"],
        1000
    );
    assert_eq!(
        value["data"]["inputSchema"]["properties"]["conversation"]["cliFlag"],
        "--conversation"
    );
    assert_eq!(
        value["data"]["inputSchema"]["properties"]["timeout"]["maximum"],
        300
    );
    assert_eq!(
        body(&run(&["schema", "get", "message.unknown"]))["error"]["code"],
        "COMMAND_NOT_FOUND"
    );
}

#[test]
fn parser_rejects_obsolete_aliases_without_echoing_supplied_secrets() {
    for args in [
        vec!["self"],
        vec!["auth", "status"],
        vec!["events", "watch"],
        vec!["profile", "add", "synthetic-secret-never-echo"],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--text",
            "synthetic-secret-never-echo",
        ],
        vec![
            "message",
            "send",
            "--conversation",
            "group:10001",
            "--reply-to",
            "synthetic-secret-never-echo",
        ],
        vec!["napcat", "forget", "synthetic-secret-never-echo"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        let error = body(&output);
        assert_eq!(error["error"]["code"], "INVALID_ARGUMENTS");
        assert!(error.get("command").is_none());
        assert!(error.get("data").is_none());
        assert!(error.get("meta").is_none());
    }
}

#[test]
fn semantic_validation_precedes_config_or_message_input_access() {
    for args in [
        vec!["message", "history"],
        vec!["message", "search", "--conversation", "group:10001"],
        vec![
            "message",
            "send",
            "--conversation",
            "group:10001",
            "--stdin",
        ],
        vec![
            "message",
            "send",
            "--conversation",
            "group:10001",
            "--request-id",
            "task-001",
        ],
        vec![
            "message",
            "send",
            "--conversation",
            "group:10001",
            "--text-file",
            "missing-input.txt",
        ],
        vec!["message", "export", "--output", "output.ndjson"],
        vec!["napcat", "uninstall", "../unrelated"],
        vec![
            "--instance",
            "other-instance",
            "napcat",
            "remove",
            "target-instance",
        ],
    ] {
        let mut args = args;
        args.extend(["--config", "missing-config.json"]);
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(body(&output)["error"]["code"], "INVALID_ARGUMENTS");
    }
}

#[test]
fn instances_require_an_explicit_creation_key_only_for_new_creation() {
    for args in [
        vec!["qq-cli", "napcat", "start", "--new"],
        vec!["qq-cli", "napcat", "start", "--request-id", "task-001"],
        vec![
            "qq-cli",
            "napcat",
            "start",
            "--new",
            "--request-id",
            "../escape",
        ],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
    assert!(Cli::try_parse_from(["qq-cli", "napcat", "start"]).is_ok());
    assert!(
        Cli::try_parse_from([
            "qq-cli",
            "napcat",
            "start",
            "--new",
            "--request-id",
            "task-001"
        ])
        .is_ok()
    );
}

#[test]
fn reply_infers_target_and_dry_run_does_not_require_an_operation_key() {
    let cli = Cli::try_parse_from([
        "qq-cli",
        "message",
        "reply",
        "m1.example.reference",
        "--text",
        "Example",
        "--dry-run",
    ])
    .unwrap();
    assert!(cli.validate().is_ok());
    assert!(
        Cli::try_parse_from([
            "qq-cli",
            "message",
            "reply",
            "m1.example.reference",
            "--conversation",
            "group:10001",
            "--text",
            "Example"
        ])
        .is_err()
    );
    for args in [
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "group:10001",
            "--image",
            "image.png",
            "--request-id",
            "task-001",
        ],
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "private:10001",
            "--text",
            "Example",
            "--dry-run",
        ],
        vec![
            "qq-cli",
            "message",
            "history",
            "--cursor",
            "c1.example.reference",
        ],
        vec![
            "qq-cli",
            "group",
            "members",
            "--cursor",
            "c1.example.reference",
        ],
    ] {
        assert!(Cli::try_parse_from(args).unwrap().validate().is_ok());
    }
}

#[test]
fn text_sources_bounds_and_explicit_identity_are_validated() {
    for args in [
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "group:0",
            "--text",
            "Example",
            "--request-id",
            "task-001",
        ],
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "10001",
            "--text",
            "Example",
            "--request-id",
            "task-001",
        ],
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "private:10001",
            "--text",
            "Example",
            "--stdin",
            "--request-id",
            "task-001",
        ],
        vec![
            "qq-cli",
            "message",
            "history",
            "--conversation",
            "group:10001",
            "--limit",
            "0",
        ],
        vec![
            "qq-cli",
            "message",
            "history",
            "--conversation",
            "group:10001",
            "--timeout",
            "301",
        ],
    ] {
        assert!(Cli::try_parse_from(args).is_err());
    }
    for (since, until) in [
        ("2026-10-01", "2026-10-02"),
        ("2026-10-02T00:00:00+08:00", "2026-10-01T00:00:00+08:00"),
    ] {
        let cli = Cli::try_parse_from([
            "qq-cli",
            "message",
            "history",
            "--conversation",
            "group:10001",
            "--since",
            since,
            "--until",
            until,
        ]);
        assert!(cli.ok().is_none_or(|cli| cli.validate().is_err()));
    }
}

#[test]
fn uninitialized_status_is_precise_and_does_not_initialize_state() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    std::fs::write(&config, r#"{"schemaVersion":1,"stateDir":"state"}"#).unwrap();
    let config = config.to_str().unwrap();
    let output = run(&["--config", config, "status"]);
    assert!(output.status.success());
    let value = body(&output);
    assert_eq!(value["data"]["state"], "not_started");
    assert!(value["data"]["account"].is_null());
    assert!(!root.path().join("state").exists());
    let installations = body(&run(&["--config", config, "napcat", "installations"]));
    assert!(installations["data"].as_array().unwrap().is_empty());
    let mut file = std::fs::File::open(root.path().join("config.json")).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert!(!bytes.is_empty());
}

#[test]
fn prepared_instances_report_unverified_state_and_never_probe_planned_endpoints() {
    let trap = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let address = trap.local_addr().unwrap();
    use qq_cli::storage::state::{Installation, Instance, Store};
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let mut transaction = store
        .transaction(Instant::now() + Duration::from_secs(5))
        .unwrap();
    transaction.registry.installations.push(Installation {
        id: "install-example".into(),
        version: "v4.18.38".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("unused-installation"),
    });
    transaction.registry.instances.push(Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: format!("http://{address}/"),
        ws_endpoint: Some(format!("ws://{address}/event")),
        credential_key: "missing-credential-never-read".into(),
        account: None,
        run_id: "run-example".into(),
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    transaction.commit().unwrap();
    let config = temp.path().join("config.json");
    std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
    let config = config.to_str().unwrap();
    let output = run(&["--config", config, "status"]);
    assert!(output.status.success());
    let value = body(&output);
    assert_eq!(value["data"]["state"], "prepared");
    assert_eq!(value["data"]["identityVerified"], false);
    assert!(value["data"]["account"].is_null());
    for args in [
        vec!["contact", "list"],
        vec!["message", "history", "--conversation", "group:20001"],
        vec!["event", "watch", "--timeout", "1"],
    ] {
        let mut command = vec!["--config", config];
        command.extend(args);
        let output = run(&command);
        assert_eq!(output.status.code(), Some(4));
        let value = body(&output);
        assert_eq!(value["error"]["code"], "INSTANCE_NOT_READY");
        let descriptor = qq_cli::cli::registry::get("status").unwrap();
        assert!(
            jsonschema::validator_for(&descriptor["inputSchema"])
                .unwrap()
                .is_valid(&value["error"]["nextAction"]["args"])
        );
    }
    assert!(!store.root().join("references").exists());
    assert_eq!(
        trap.accept().err().unwrap().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn every_schema_compiles_and_flags_are_present_in_offline_help() {
    for entry in qq_cli::cli::registry::list() {
        let name = entry.name.as_str();
        let schema = qq_cli::cli::registry::get(name).unwrap();
        for field in ["inputSchema", "responseSchema", "preStreamErrorSchema"] {
            if let Some(value) = schema.get(field) {
                jsonschema::validator_for(value).unwrap_or_else(|e| panic!("{name} {field}: {e}"));
            }
        }
        let mut args = name.split('.').collect::<Vec<_>>();
        args.push("--help");
        let output = run(&args);
        let help = String::from_utf8(output.stdout).unwrap();
        for property in schema["inputSchema"]["properties"]
            .as_object()
            .unwrap()
            .values()
        {
            if let Some(flag) = property["cliFlag"].as_str() {
                assert!(help.contains(flag), "{name} missing {flag}");
            }
        }
    }
    // Resume dependencies must agree between the parser and exported JSON Schema.
    let watch = qq_cli::cli::registry::get("event.watch").unwrap();
    let validator = jsonschema::validator_for(&watch["inputSchema"]).unwrap();
    assert!(!validator.is_valid(&serde_json::json!({"resume":"synthetic-checkpoint"})));
    assert!(
        Cli::try_parse_from([
            "qq-cli",
            "event",
            "watch",
            "--resume",
            "synthetic-checkpoint"
        ])
        .is_err()
    );
    assert!(validator.is_valid(&serde_json::json!({"conversation":"private:10001","resume":"synthetic-checkpoint","timeout":120})));
}

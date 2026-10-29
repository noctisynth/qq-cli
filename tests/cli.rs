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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn macos_doctor_describes_managed_preparation_without_creating_state_or_claiming_compatibility() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let config = root.join("config.json");
    std::fs::write(&config, b"{}").unwrap();
    let output = run(&["--config", config.to_str().unwrap(), "doctor"]);
    assert!(output.status.success());
    let value = body(&output);
    let data = &value["data"];
    assert_eq!(data["host"]["nativeArch"], "aarch64");
    assert_eq!(data["release"]["version"], "v4.18.41");
    assert_eq!(data["canStart"], false);
    let checks = data["checks"].as_array().unwrap();
    let qq = checks.iter().find(|v| v["name"] == "managed_qq").unwrap();
    assert_eq!(qq["state"], "required");
    assert_eq!(qq["code"], "QQ_PREPARATION_REQUIRED");
    assert_eq!(qq["qqBuild"], "53644");
    assert!(!checks.iter().any(|v| v["name"] == "qq"));
    assert_eq!(
        checks
            .iter()
            .find(|v| v["name"] == "compatibility")
            .unwrap()["result"]["state"],
        "unknown"
    );
    let schema = qq_cli::cli::registry::get("doctor").unwrap();
    assert!(
        jsonschema::validator_for(&schema["responseSchema"])
            .unwrap()
            .is_valid(&value)
    );
    assert!(
        !serde_json::to_string(&value)
            .unwrap()
            .contains(root.to_str().unwrap())
    );
    assert!(!root.join("state").exists());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let original_key = checks
        .iter()
        .find(|v| v["name"] == "compatibility")
        .unwrap()["result"]["keyFingerprint"]
        .clone();
    let older = body(&run(&[
        "--config",
        config.to_str().unwrap(),
        "doctor",
        "--version",
        "4.18.38",
    ]));
    assert_eq!(older["data"]["release"]["version"], "v4.18.38");
    assert_eq!(older["data"]["canStart"], false);
    let older_compatibility = older["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "compatibility")
        .unwrap();
    assert_eq!(older_compatibility["result"]["state"], "unknown");
    assert_ne!(
        older_compatibility["result"]["keyFingerprint"],
        original_key
    );
    assert!(!root.join("state").exists());
    let output = run(&[
        "--config",
        config.to_str().unwrap(),
        "doctor",
        "--version",
        "v4.18.37",
    ]);
    assert!(output.status.success());
    let value = body(&output);
    assert!(
        value["data"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["name"] == "compatibility" && v["result"]["state"] == "unsupported")
    );
    assert!(!root.join("state").exists());
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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn macos_version_limits_refuse_before_credentials_and_keep_the_registered_run() {
    use qq_cli::storage::state::{Installation, Instance, Store};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let store = Store::new(root.join("state"));
    let mut tx = store
        .transaction(Instant::now() + Duration::from_secs(5))
        .unwrap();
    tx.registry.installations.push(Installation {
        id: "install-example".into(),
        version: "v4.18.41".into(),
        digest: "a".repeat(64),
        directory: root.join("untrusted-installation"),
    });
    tx.registry.instances.push(Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "must-not-read".into(),
        account: None,
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let original = std::fs::read(store.root().join("registry.json")).unwrap();
    let config = root.join("config.json");
    std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
    for (args, code) in [
        (
            vec!["napcat", "update", "--version", "4.18.37"],
            "VERSION_UNSUPPORTED",
        ),
        (
            vec!["napcat", "update", "--version", "4.18.41"],
            "VERSION_UNSUPPORTED",
        ),
        (vec!["napcat", "rollback"], "ROLLBACK_UNAVAILABLE"),
    ] {
        let mut command = vec!["--config", config.to_str().unwrap()];
        command.extend(args);
        let output = run(&command);
        assert_eq!(body(&output)["error"]["code"], code);
        assert_eq!(
            std::fs::read(store.root().join("registry.json")).unwrap(),
            original
        );
        for name in [
            "references",
            "macos-stops",
            "run-transitions",
            "start-requests",
        ] {
            assert!(!store.root().join(name).exists());
        }
    }
    let update = qq_cli::cli::registry::get("napcat.update").unwrap();
    let timeout = &update["inputSchema"]["properties"]["timeout"];
    assert_eq!(timeout["default"], 900);
    assert_eq!(timeout["maximum"], 1800);
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
        let unsupported_remove =
            !cfg!(windows) && command.as_slice() == ["napcat", "remove", "missing-instance"];
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
                || (unsupported_remove && value["error"]["code"] == "PLATFORM_UNSUPPORTED")
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
    assert_eq!(commands.len(), 115);
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

#[test]
fn every_root_group_and_leaf_help_entry_is_available_offline() {
    use clap::CommandFactory;
    fn visit(command: &clap::Command, prefix: &[&str]) -> usize {
        let mut args = prefix.to_vec();
        args.push("--help");
        let output = run(&args);
        assert!(output.status.success(), "{args:?}");
        assert!(output.stderr.is_empty());
        let mut count = 1;
        for child in command.get_subcommands().filter(|c| c.get_name() != "help") {
            let mut path = prefix.to_vec();
            path.push(child.get_name());
            count += visit(child, &path);
        }
        count
    }
    let mut command = Cli::command();
    command.build();
    assert_eq!(visit(&command, &[]), 132);
    assert_eq!(qq_cli::cli::registry::list().len(), 115);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn macos_start_has_its_own_preparation_budget_and_missing_installation_is_actionable_without_state_creation()
 {
    let cli = Cli::try_parse_from(["qq-cli", "napcat", "start"]).unwrap();
    assert_eq!(cli.command.timeout_seconds(), 900);
    assert!(Cli::try_parse_from(["qq-cli", "napcat", "start", "--timeout", "1800"]).is_ok());
    assert!(Cli::try_parse_from(["qq-cli", "napcat", "start", "--timeout", "1801"]).is_err());
    let schema = qq_cli::cli::registry::get("napcat.start").unwrap();
    assert_eq!(
        schema["inputSchema"]["properties"]["timeout"]["default"],
        900
    );
    assert_eq!(
        schema["inputSchema"]["properties"]["timeout"]["maximum"],
        1800
    );
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let config = temp.path().join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&serde_json::json!({"stateDir":state})).unwrap(),
    )
    .unwrap();
    let result = body(&run(&[
        "--config",
        config.to_str().unwrap(),
        "napcat",
        "start",
    ]));
    assert_eq!(result["error"]["code"], "INSTALLATION_NOT_FOUND");
    assert!(!state.exists());
}

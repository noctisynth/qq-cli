use std::io;
use std::net::TcpListener;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

fn run(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .env("QQ_CLI_TOKEN", "synthetic-secret-never-echo")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn CLI");
    // Keep stdin open: M0 must not wait for input, including --stdin.
    let input = child.stdin.take().expect("piped stdin");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().expect("poll CLI").is_some() {
            break;
        }
        if Instant::now() > deadline {
            child.kill().expect("stop hung CLI");
            let _ = child.wait();
            panic!("CLI blocked on offline input");
        }
        thread::sleep(Duration::from_millis(10));
    }
    drop(input);
    child.wait_with_output().expect("CLI output")
}

fn failure(output: &Output, exit_code: i32, code: &str, command: &str) -> Value {
    assert_eq!(output.status.code(), Some(exit_code));
    assert!(output.stderr.is_empty(), "no default stderr diagnostics");
    let text = std::str::from_utf8(&output.stdout).expect("UTF-8 output");
    assert_eq!(text.lines().count(), 1, "one JSON envelope");
    let body: Value = serde_json::from_str(text).expect("JSON envelope");
    assert_eq!(body["schemaVersion"], 1);
    assert_eq!(body["ok"], false);
    assert!(body["data"].is_null());
    assert_eq!(body["command"], command);
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["outcome"], "not_attempted");
    assert!(
        body["meta"]["invocationId"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );
    assert!(!text.contains("synthetic-secret-never-echo"));
    body
}

#[test]
fn help_and_version_work_offline_at_each_command_level() {
    for args in [
        vec!["--help"],
        vec!["message", "--help"],
        vec!["message", "get", "--help"],
        vec!["message", "history", "--help"],
        vec!["message", "send", "--help"],
    ] {
        let output = run(&args);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    let output = run(&["--version"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("version"),
        format!("qq-cli {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn malformed_commands_are_json_and_do_not_echo_supplied_values() {
    for args in [
        vec![],
        vec!["synthetic-secret-never-echo"],
        vec!["message"],
        vec!["status", "--token", "synthetic-secret-never-echo"],
        vec!["message", "get", "--id", "synthetic-secret-never-echo"],
        vec!["message", "history", "--user", "0"],
        vec!["message", "history", "--user", "abc"],
        vec!["message", "history", "--user", "10001", "--limit", "0"],
        vec!["message", "history", "--user", "10001", "--limit", "101"],
    ] {
        failure(&run(&args), 2, "INVALID_ARGUMENTS", "cli");
    }
}

#[test]
fn every_business_command_is_explicitly_unimplemented() {
    for (args, name) in [
        (vec!["doctor"], "doctor"),
        (vec!["status"], "status"),
        (vec!["self"], "self"),
        (
            vec!["message", "get", "--id", "-9007199254740993"],
            "message.get",
        ),
        (
            vec!["message", "history", "--user", "9007199254740993"],
            "message.history",
        ),
    ] {
        let body = failure(&run(&args), 6, "NOT_IMPLEMENTED", name);
        assert!(body["meta"].get("requestId").is_none());
    }
}

#[test]
fn send_accepts_each_input_source_without_reading_it() {
    for input in [
        vec![
            "--text",
            "synthetic-secret-never-echo\n[CQ:image,file=synthetic]",
        ],
        vec!["--text-file", "missing-synthetic-input.txt"],
        vec!["--stdin"],
    ] {
        let mut args = vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "test-request",
        ];
        args.extend(input);
        let output = run(&args);
        let body = failure(&output, 6, "NOT_IMPLEMENTED", "message.send");
        assert_eq!(body["meta"]["requestId"], "test-request");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("missing-synthetic-input.txt"));
    }
}

#[test]
fn send_requires_target_key_and_exactly_one_text_source() {
    for args in [
        vec!["message", "send", "--user", "10001", "--request-id", "k"],
        vec!["message", "send", "--user", "10001", "--text", "test"],
        vec!["message", "send", "--request-id", "k", "--text", "test"],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "k",
            "--text",
            "test",
            "--stdin",
        ],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "k",
            "--stdin",
            "--text-file",
            "missing.txt",
        ],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "bad/key",
            "--text",
            "test",
        ],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "k",
            "--text",
            "",
        ],
    ] {
        failure(&run(&args), 2, "INVALID_ARGUMENTS", "cli");
    }
    let long_key = "a".repeat(129);
    failure(
        &run(&[
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            &long_key,
            "--stdin",
        ]),
        2,
        "INVALID_ARGUMENTS",
        "cli",
    );
}

#[test]
fn config_flag_is_global_and_does_not_open_files() {
    for args in [
        vec!["--config", "missing-config.json", "status"],
        vec!["status", "--config", "missing-config.json"],
        vec![
            "message",
            "get",
            "--id",
            "1",
            "--config",
            "missing-config.json",
        ],
    ] {
        let name = if args[0] == "message" {
            "message.get"
        } else {
            "status"
        };
        failure(&run(&args), 6, "NOT_IMPLEMENTED", name);
    }
}

#[test]
fn business_commands_do_not_connect_to_a_configured_endpoint() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fake backend");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let endpoint = format!(
        "http://{}",
        listener.local_addr().expect("listener address")
    );
    for args in [
        vec!["doctor"],
        vec!["status"],
        vec!["self"],
        vec!["message", "get", "--id", "1"],
        vec!["message", "history", "--user", "10001"],
        vec![
            "message",
            "send",
            "--user",
            "10001",
            "--request-id",
            "k",
            "--text",
            "test",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .env("QQ_CLI_ENDPOINT", &endpoint)
            .env("QQ_CLI_TOKEN", "synthetic-secret-never-echo")
            .stdin(Stdio::null())
            .output()
            .expect("offline command");
        assert_eq!(output.status.code(), Some(6));
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn invocation_ids_change_between_processes() {
    let first = failure(&run(&["status"]), 6, "NOT_IMPLEMENTED", "status");
    let second = failure(&run(&["status"]), 6, "NOT_IMPLEMENTED", "status");
    assert_ne!(
        first["meta"]["invocationId"],
        second["meta"]["invocationId"]
    );
}

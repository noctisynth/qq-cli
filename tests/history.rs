use std::cell::RefCell;
use std::time::Instant;

use clap::Parser;
use qq_cli::cli::args::Cli;
use qq_cli::cli::args::{Command, ExportArgs, MessageCommand};
use qq_cli::core::error::{AppError, Result};
use qq_cli::messaging::history;
use qq_cli::messaging::session::Session;
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use qq_cli::storage::state::Instance;
use qq_cli::transport::client::{Action, Backend};
use serde_json::{Value, json};

struct Fake {
    calls: RefCell<Vec<(String, Value)>>,
    private: bool,
    fail_old: bool,
    repeat: bool,
}
impl Fake {
    fn raw(&self, id: u64) -> Value {
        let mut value = json!({"message_id":id.to_string(),"real_seq":id.to_string(),"time":1700000000+id,
            "self_id":"10001","user_id":"10002","message":[{"type":"text","data":{"text":format!("sample {id}")}}]});
        if self.private {
            value["message_type"] = json!("private");
            value["target_id"] = json!("10001");
        } else {
            value["message_type"] = json!("group");
            value["group_id"] = json!("20001");
        }
        value
    }
}
impl Backend for Fake {
    fn call(&self, action: Action, args: &Value, _: Instant) -> Result<Value> {
        self.calls
            .borrow_mut()
            .push((action.name().into(), args.clone()));
        Ok(match action {
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::Status => json!({"online":true}),
            Action::LoginInfo => json!({"user_id":"10001"}),
            Action::Message => self.raw(args["message_id"].as_str().unwrap().parse().unwrap()),
            Action::FriendHistory | Action::GroupHistory => {
                assert_eq!(matches!(action, Action::FriendHistory), self.private);
                assert_eq!(args["disable_get_url"], true);
                assert_eq!(args["parse_mult_msg"], false);
                let anchor = args["message_seq"].as_str();
                if anchor.is_some() && self.fail_old {
                    return Err(AppError::new("BACKEND_REJECTED", "Synthetic failure", 4));
                }
                let ids: Vec<u64> = if self.repeat {
                    vec![5, 4, 3]
                } else {
                    match anchor {
                        None => vec![5, 4, 3],
                        Some("3") => vec![3, 2, 1],
                        _ => vec![],
                    }
                };
                json!({"messages":ids.iter().map(|id|self.raw(*id)).collect::<Vec<_>>()})
            }
            _ => panic!("unexpected synthetic action"),
        })
    }
}
fn instance() -> Instance {
    Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}
fn run(args: Vec<&str>, session: &Session<'_, Fake>, refs: &References) -> Value {
    let cli = Cli::try_parse_from(args).unwrap();
    cli.validate().unwrap();
    let (args, query, search) = history::request(&cli.command).unwrap();
    let value = serde_json::to_value(history::execute(args, query, search, session, refs).unwrap())
        .unwrap();
    let descriptor = qq_cli::cli::registry::get(if search {
        "message.search"
    } else {
        "message.history"
    })
    .unwrap();
    assert!(
        jsonschema::validator_for(&descriptor["responseSchema"])
            .unwrap()
            .is_valid(&value),
        "{value}"
    );
    let validator = jsonschema::validator_for(&descriptor["responseSchema"]).unwrap();
    let mut invalid = value.clone();
    invalid["meta"]["scope"]["account"] = json!(10001);
    assert!(!validator.is_valid(&invalid));
    let mut invalid = value.clone();
    invalid["pageInfo"]["coverage"] = json!("all_qq_history");
    assert!(!validator.is_valid(&invalid));
    if value["pageInfo"]["complete"] == false {
        let mut invalid = value.clone();
        invalid["pageInfo"]
            .as_object_mut()
            .unwrap()
            .remove("nextCursor");
        assert!(!validator.is_valid(&invalid));
    }
    if value["ok"] == false {
        let mut invalid = value.clone();
        invalid.as_object_mut().unwrap().remove("error");
        assert!(!validator.is_valid(&invalid));
    }
    value
}

#[test]
fn history_transport_fallback_keeps_anchor_and_counts_failed_attempts_in_budget() {
    struct Flaky {
        base: Fake,
        mode: &'static str,
        attempts: RefCell<Vec<Value>>,
    }
    impl Backend for Flaky {
        fn call(&self, action: Action, args: &Value, deadline: Instant) -> Result<Value> {
            if matches!(action, Action::GroupHistory) {
                self.attempts.borrow_mut().push(args.clone());
                if self.mode == "auth" {
                    return Err(AppError::new("BACKEND_AUTH", "Synthetic auth refusal", 3));
                }
                if self.mode != "recover" || args["count"].as_u64().unwrap() > 5 {
                    return Err(AppError::new(
                        "BACKEND_TRANSPORT",
                        "Synthetic read timeout",
                        3,
                    ));
                }
            }
            self.base.call(action, args, deadline)
        }
    }
    for mode in ["recover", "exhausted", "budget", "auth"] {
        let temp = tempfile::tempdir().unwrap();
        let refs = References::with_key(temp.path().join("refs"), [59; 32]);
        let backend = Flaky {
            base: Fake {
                calls: RefCell::new(vec![]),
                private: false,
                fail_old: false,
                repeat: false,
            },
            mode,
            attempts: RefCell::new(vec![]),
        };
        let mut config = Config::default();
        if mode == "budget" {
            config.page_limit = 2;
        }
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let cli = Cli::try_parse_from([
            "qq-cli",
            "message",
            "history",
            "--conversation",
            "group:20001",
            "--limit",
            "2",
        ])
        .unwrap();
        let (args, query, search) = history::request(&cli.command).unwrap();
        let first =
            serde_json::to_value(history::execute(args, query, search, &session, &refs).unwrap())
                .unwrap();
        let expected: &[u64] = match mode {
            "recover" => &[20, 10, 5],
            "budget" => &[20, 10],
            "auth" => &[20],
            _ => &[20, 10, 5, 2],
        };
        assert_eq!(
            backend
                .attempts
                .borrow()
                .iter()
                .map(|v| v["count"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            backend
                .attempts
                .borrow()
                .iter()
                .all(|v| v.get("message_seq").is_none())
        );
        assert_eq!(first["pageInfo"]["backendPages"], expected.len());
        assert_eq!(first["pageInfo"]["complete"], false);
        if mode == "recover" {
            assert_eq!(first["data"][0]["id"], "5");
            assert_eq!(first["data"][1]["id"], "4");
            backend.attempts.borrow_mut().clear();
            let cli = Cli::try_parse_from([
                "qq-cli",
                "message",
                "history",
                "--cursor",
                first["pageInfo"]["nextCursor"].as_str().unwrap(),
                "--limit",
                "3",
            ])
            .unwrap();
            let (args, query, search) = history::request(&cli.command).unwrap();
            let second = serde_json::to_value(
                history::execute(args, query, search, &session, &refs).unwrap(),
            )
            .unwrap();
            assert_eq!(
                second["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v["id"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                ["3", "2", "1"]
            );
            assert!(
                backend
                    .attempts
                    .borrow()
                    .iter()
                    .all(|v| v["message_seq"] == "3")
            );
        } else {
            assert_eq!(
                first["pageInfo"]["stopReason"],
                if mode == "budget" {
                    "page_limit"
                } else {
                    "backend_error"
                }
            );
            assert!(first["pageInfo"]["nextCursor"].is_string());
            assert!(first["data"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn private_and_group_history_resume_without_duplicates_or_bodies_in_state() {
    for private in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let refs = References::with_key(temp.path().join("refs"), [51; 32]);
        let backend = Fake {
            calls: RefCell::new(vec![]),
            private,
            fail_old: false,
            repeat: false,
        };
        let config = Config::default();
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let target = if private {
            "private:10002"
        } else {
            "group:20001"
        };
        let first = run(
            vec![
                "qq-cli",
                "message",
                "history",
                "--conversation",
                target,
                "--limit",
                "2",
            ],
            &session,
            &refs,
        );
        assert_eq!(first["ok"], true);
        assert_eq!(first["data"][0]["id"], "5");
        let second = run(
            vec![
                "qq-cli",
                "message",
                "history",
                "--cursor",
                first["pageInfo"]["nextCursor"].as_str().unwrap(),
                "--limit",
                "2",
            ],
            &session,
            &refs,
        );
        assert_eq!(second["data"][0]["id"], "3");
        assert_eq!(second["data"][1]["id"], "2");
        let third = run(
            vec![
                "qq-cli",
                "message",
                "history",
                "--cursor",
                second["pageInfo"]["nextCursor"].as_str().unwrap(),
                "--limit",
                "2",
            ],
            &session,
            &refs,
        );
        assert_eq!(third["data"][0]["id"], "1");
        assert_eq!(third["pageInfo"]["complete"], true);
        assert_eq!(third["pageInfo"]["stopReason"], "backend_end");
        for file in std::fs::read_dir(temp.path().join("refs")).unwrap() {
            assert!(
                !std::fs::read_to_string(file.unwrap().path())
                    .unwrap()
                    .contains("sample ")
            );
        }
    }
}

#[test]
fn search_restores_query_and_filters_time_without_claiming_full_archive() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [52; 32]);
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let first = run(
        vec![
            "qq-cli",
            "message",
            "search",
            "--conversation",
            "group:20001",
            "--query",
            "sample",
            "--until",
            "2023-11-14T22:13:24Z",
            "--limit",
            "1",
        ],
        &session,
        &refs,
    );
    assert_eq!(first["data"][0]["id"], "3");
    let cli = Cli::try_parse_from([
        "qq-cli",
        "message",
        "search",
        "--cursor",
        first["pageInfo"]["nextCursor"].as_str().unwrap(),
    ])
    .unwrap();
    assert_eq!(
        history::inferred_instance(&cli.command, &refs)
            .unwrap()
            .as_deref(),
        Some("instance-example")
    );
    let (args, query, search) = history::request(&cli.command).unwrap();
    let rest =
        serde_json::to_value(history::execute(args, query, search, &session, &refs).unwrap())
            .unwrap();
    assert_eq!(rest["data"].as_array().unwrap().len(), 2);
    assert_eq!(rest["pageInfo"]["coverage"], "backend_accessible");
}

#[test]
fn rejection_no_progress_and_scan_budget_are_explicit_partial_results() {
    for (fail_old, repeat, scan_limit, expected) in [
        (true, false, 1000, "backend_error"),
        (false, true, 1000, "no_progress"),
        (false, false, 2, "scan_limit"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let refs = References::with_key(temp.path().join("refs"), [53; 32]);
        let backend = Fake {
            calls: RefCell::new(vec![]),
            private: false,
            fail_old,
            repeat,
        };
        let config = Config {
            scan_limit,
            ..Config::default()
        };
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let page = run(
            vec![
                "qq-cli",
                "message",
                "history",
                "--conversation",
                "group:20001",
            ],
            &session,
            &refs,
        );
        assert_eq!(page["error"]["code"], "PARTIAL_RESULT");
        assert_eq!(page["pageInfo"]["stopReason"], expected);
        assert_eq!(page["pageInfo"]["complete"], false);
        assert!(page["pageInfo"]["nextCursor"].is_string());
    }
}

#[test]
fn continuation_rejects_conflicts_and_restart_before_reading_domain_data() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [54; 32]);
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let config = Config::default();
    let original = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &original,
        expected_version: "4.18.38",
    };
    let page = run(
        vec![
            "qq-cli",
            "message",
            "history",
            "--conversation",
            "group:20001",
            "--limit",
            "1",
        ],
        &session,
        &refs,
    );
    let token = page["pageInfo"]["nextCursor"].as_str().unwrap();
    let cli = Cli::try_parse_from([
        "qq-cli",
        "message",
        "history",
        "--cursor",
        token,
        "--conversation",
        "group:20002",
    ])
    .unwrap();
    backend.calls.borrow_mut().clear();
    let (args, query, search) = history::request(&cli.command).unwrap();
    assert_eq!(
        history::execute(args, query, search, &session, &refs)
            .err()
            .unwrap()
            .code,
        "CURSOR_CONFLICT"
    );
    assert!(backend.calls.borrow().is_empty());
    let mut restarted = original.clone();
    restarted.run_id = "restarted-run".into();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &restarted,
        expected_version: "4.18.38",
    };
    let cli = Cli::try_parse_from(["qq-cli", "message", "history", "--cursor", token]).unwrap();
    let (args, query, search) = history::request(&cli.command).unwrap();
    assert_eq!(
        history::execute(args, query, search, &session, &refs)
            .err()
            .unwrap()
            .code,
        "CURSOR_STALE"
    );
    assert!(backend.calls.borrow().is_empty());
}

fn export_args(path: &std::path::Path, resume: bool) -> ExportArgs {
    let mut values = vec![
        "qq-cli",
        "message",
        "export",
        "--output",
        path.to_str().unwrap(),
        "--limit",
        "2",
    ];
    if resume {
        values.push("--resume");
    } else {
        values.extend(["--conversation", "group:20001"]);
    }
    let cli = Cli::try_parse_from(values).unwrap();
    cli.validate().unwrap();
    let Command::Message {
        command: MessageCommand::Export(args),
    } = cli.command
    else {
        panic!()
    };
    args
}

#[test]
fn export_atomic_checkpoints_resume_without_duplicates_and_stdout_has_no_bodies() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [71; 32]);
    let output = temp.path().join("messages.ndjson");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let first = serde_json::to_value(
        qq_cli::messaging::export::execute(&export_args(&output, false), &session, &refs).unwrap(),
    )
    .unwrap();
    assert_eq!(first["data"]["count"], 2);
    assert!(!serde_json::to_string(&first).unwrap().contains("sample "));
    assert_eq!(
        qq_cli::messaging::export::inferred_instance(&export_args(&output, true), &refs)
            .unwrap()
            .as_deref(),
        Some("instance-example")
    );
    qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs).unwrap();
    let final_page = serde_json::to_value(
        qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs).unwrap(),
    )
    .unwrap();
    assert_eq!(final_page["data"]["count"], 5);
    assert_eq!(final_page["data"]["complete"], true);
    let contents = std::fs::read_to_string(&output).unwrap();
    let records = contents
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 6);
    assert_eq!(records[5]["type"], "checkpoint");
    assert_eq!(
        records[..5]
            .iter()
            .map(|v| v["message"]["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["5", "4", "3", "2", "1"]
    );
    qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs).unwrap();
    assert_eq!(std::fs::read_to_string(output).unwrap(), contents);
    for file in std::fs::read_dir(temp.path().join("refs")).unwrap() {
        assert!(
            !std::fs::read_to_string(file.unwrap().path())
                .unwrap()
                .contains("sample ")
        );
    }
}

#[test]
fn export_rejects_overwrite_query_conflict_restart_and_modified_body_before_domain_reads() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [72; 32]);
    let output = temp.path().join("messages.ndjson");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let config = Config::default();
    let original = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &original,
        expected_version: "4.18.38",
    };
    qq_cli::messaging::export::execute(&export_args(&output, false), &session, &refs).unwrap();
    backend.calls.borrow_mut().clear();
    assert_eq!(
        qq_cli::messaging::export::execute(&export_args(&output, false), &session, &refs)
            .err()
            .unwrap()
            .code,
        "OUTPUT_EXISTS"
    );
    let mut args = export_args(&output, true);
    args.query = Some("other".into());
    assert_eq!(
        qq_cli::messaging::export::execute(&args, &session, &refs)
            .err()
            .unwrap()
            .code,
        "CURSOR_CONFLICT"
    );
    let mut restarted = original.clone();
    restarted.run_id = "new-run".into();
    let changed = Session {
        backend: &backend,
        config: &config,
        instance: &restarted,
        expected_version: "4.18.38",
    };
    assert_eq!(
        qq_cli::messaging::export::execute(&export_args(&output, true), &changed, &refs)
            .err()
            .unwrap()
            .code,
        "CURSOR_STALE"
    );
    let modified = std::fs::read_to_string(&output)
        .unwrap()
        .replace("sample ", "edited ");
    std::fs::write(&output, &modified).unwrap();
    assert_eq!(
        qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs)
            .err()
            .unwrap()
            .code,
        "EXPORT_MODIFIED"
    );
    assert_eq!(std::fs::read_to_string(output).unwrap(), modified);
    assert!(backend.calls.borrow().is_empty());
}

#[test]
fn export_checkpoint_is_state_bound_and_permission_denial_keeps_the_previous_output() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [73; 32]);
    let output = temp.path().join("messages.ndjson");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let mut config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    qq_cli::messaging::export::execute(&export_args(&output, false), &session, &refs).unwrap();
    let before = std::fs::read(&output).unwrap();
    let foreign = References::with_key(temp.path().join("refs"), [74; 32]);
    assert_eq!(
        qq_cli::messaging::export::inferred_instance(&export_args(&output, true), &foreign)
            .err()
            .unwrap()
            .code,
        "REFERENCE_INVALID"
    );
    config.permissions.read = false;
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert_eq!(
        qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs)
            .err()
            .unwrap()
            .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(std::fs::read(output).unwrap(), before);
}

#[test]
fn export_directory_lock_has_a_deadline_and_never_overwrites_a_foreign_marker() {
    use std::time::Duration;
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [75; 32]);
    let output = temp.path().join("messages.ndjson");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let marker = temp.path().join(".qq-cli-export.lock");
    std::fs::write(&marker, b"user-owned marker").unwrap();
    assert_eq!(
        qq_cli::messaging::export::execute(&export_args(&output, false), &session, &refs)
            .err()
            .unwrap()
            .code,
        "OUTPUT_CONFLICT"
    );
    assert_eq!(std::fs::read(&marker).unwrap(), b"user-owned marker");
    std::fs::write(&marker, b"qq-cli export lock v1\n").unwrap();
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&marker)
        .unwrap();
    lock.try_lock().unwrap();
    let mut args = export_args(&output, false);
    args.deadline.timeout = 1;
    let start = Instant::now();
    assert_eq!(
        qq_cli::messaging::export::execute(&args, &session, &refs)
            .err()
            .unwrap()
            .code,
        "EXPORT_BUSY"
    );
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(!output.exists());
    assert!(backend.calls.borrow().is_empty());
}

#[test]
fn partial_export_persists_a_resumable_checkpoint_without_repeating_written_messages() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [76; 32]);
    let output = temp.path().join("messages.ndjson");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: true,
        repeat: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut args = export_args(&output, false);
    args.limit = 4;
    let result = qq_cli::messaging::export::execute(&args, &session, &refs).unwrap();
    assert_eq!(result.exit_code(), 7);
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["data"]["count"], 3);
    assert_eq!(result["pageInfo"]["stopReason"], "backend_error");
    let backend = Fake {
        calls: RefCell::new(vec![]),
        private: false,
        fail_old: false,
        repeat: false,
    };
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs).unwrap();
    let result = serde_json::to_value(
        qq_cli::messaging::export::execute(&export_args(&output, true), &session, &refs).unwrap(),
    )
    .unwrap();
    assert_eq!(result["data"]["count"], 5);
    assert_eq!(result["data"]["complete"], true);
    assert_eq!(std::fs::read_to_string(output).unwrap().lines().count(), 6);
}

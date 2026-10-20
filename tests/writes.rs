use std::cell::Cell;
use std::time::{Duration, Instant};

use qq_cli::core::domain::Conversation;
use qq_cli::core::error::{AppError, Result};
use qq_cli::messaging::session::Session;
use qq_cli::messaging::write::{self, Plan};
use qq_cli::requests::ledger::{self, Outcome};
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use qq_cli::storage::state::{Instance, Store};
use qq_cli::transport::client::{Action, Backend};
use serde_json::{Value, json};

struct Fake {
    calls: Cell<usize>,
    writes: Cell<usize>,
    identity: Cell<usize>,
    mode: &'static str,
}
impl Backend for Fake {
    fn call(&self, action: Action, args: &Value, _: Instant) -> Result<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.mode == "offline" {
            return Err(AppError::new("BACKEND_TRANSPORT", "Synthetic offline", 3));
        }
        Ok(match action {
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::Status => json!({"online":true}),
            Action::LoginInfo => {
                self.identity.set(self.identity.get() + 1);
                json!({"user_id":if self.mode=="changed" && self.identity.get()>1 {"10002"} else {"10001"}})
            }
            Action::SendGroup => {
                self.writes.set(self.writes.get() + 1);
                if let Some(root) = std::env::var_os("QQ_CLI_TEST_REQUEST_ROOT") {
                    use std::io::Write;
                    let mut log = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(std::path::PathBuf::from(root).join("attempts.txt"))
                        .unwrap();
                    log.write_all(b"attempt\n").unwrap();
                    log.sync_all().unwrap();
                    if self.mode == "crash" {
                        std::process::exit(77);
                    }
                }
                assert_eq!(args["group_id"], "20001");
                assert_eq!(args["auto_escape"], true);
                for segment in args["message"].as_array().unwrap() {
                    if matches!(segment["type"].as_str(), Some("image" | "file")) {
                        let path = std::path::Path::new(segment["data"]["file"].as_str().unwrap());
                        assert_eq!(std::fs::read(path).unwrap(), b"synthetic attachment bytes");
                        let state = path.parent().unwrap().parent().unwrap().parent().unwrap();
                        assert_eq!(
                            ledger::get(
                                &Store::new(state.to_owned()),
                                if self.mode == "reply" {
                                    "reply-key"
                                } else {
                                    "attachment-key"
                                }
                            )
                            .unwrap()
                            .outcome,
                            Outcome::Pending
                        );
                        assert!(
                            path.parent()
                                .unwrap()
                                .join(".qq-cli-attachment.json")
                                .is_file()
                        );
                    }
                }
                assert_eq!(
                    args["message"][0]["type"],
                    if self.mode == "reply" {
                        "reply"
                    } else {
                        "text"
                    }
                );
                match self.mode {
                    "timeout" | "attachment_timeout" => {
                        return Err(AppError::new(
                            "BACKEND_TRANSPORT",
                            "Synthetic lost acknowledgement",
                            3,
                        ));
                    }
                    "business" => {
                        return Err(AppError::new(
                            "BACKEND_REJECTED",
                            "Synthetic error after possible side effects",
                            4,
                        ));
                    }
                    "auth" => {
                        return Err(AppError::new("BACKEND_AUTH", "Synthetic auth refusal", 3));
                    }
                    "missing_id" => json!({}),
                    "zero_id" => json!({"message_id":"-0"}),
                    "oversized_id" => json!({"message_id":"18446744073709551616"}),
                    _ => json!({"message_id":"9007199254740993"}),
                }
            }
            Action::Message => {
                json!({"message_id":"40001","real_seq":"101","time":1700000000,"message_type":"group",
                "group_id":"20001","self_id":"10001","user_id":"10002","message":[{"type":"text","data":{"text":"synthetic original"}}]})
            }
            Action::GroupMembers => {
                assert_eq!(args, &json!({"group_id":"20001","no_cache":true}));
                match self.mode {
                    "absent" => json!([]),
                    "bad_member" => json!([{"user_id":null}]),
                    _ => json!([{"user_id":"10002"}]),
                }
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
fn fake(mode: &'static str) -> Fake {
    Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        identity: Cell::new(0),
        mode,
    }
}
fn plan() -> Plan {
    Plan {
        conversation: Conversation::Group("20001".into()),
        segments: vec![json!({"type":"text","data":{"text":"[CQ:at,qq=all] synthetic literal"}})],
        reply: None,
        attachments: vec![],
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn mentions_are_checked_before_writes_and_dry_runs_do_not_create_intents() {
    for (mode, expected) in [
        ("ok", None),
        ("absent", Some("MENTION_NOT_MEMBER")),
        ("bad_member", Some("BACKEND_PROTOCOL")),
        ("changed", Some("ACCOUNT_CHANGED")),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let config = Config::default();
        let instance = instance();
        for dry_run in [true, false] {
            let backend = fake(mode);
            let session = Session {
                backend: &backend,
                config: &config,
                instance: &instance,
                expected_version: "4.18.38",
            };
            let mut message_plan = plan();
            message_plan
                .segments
                .push(json!({"type":"at","data":{"qq":"10002"}}));
            let result = write::execute(
                message_plan,
                Some("mention-check"),
                dry_run,
                &session,
                &store,
                None,
                deadline(),
            );
            match expected {
                None => {
                    result.unwrap();
                }
                Some(code) => assert_eq!(result.err().unwrap().code, code),
            }
            assert_eq!(
                backend.writes.get(),
                usize::from(expected.is_none() && !dry_run)
            );
            if dry_run {
                assert!(!store.root().exists());
            }
            if !dry_run && expected.is_none() {
                let absent = fake("absent");
                let cached_session = Session {
                    backend: &absent,
                    config: &config,
                    instance: &instance,
                    expected_version: "4.18.38",
                };
                let mut same = plan();
                same.segments
                    .push(json!({"type":"at","data":{"qq":"10002"}}));
                let cached = write::execute(
                    same,
                    Some("mention-check"),
                    false,
                    &cached_session,
                    &store,
                    None,
                    deadline(),
                )
                .unwrap();
                assert_eq!(
                    serde_json::to_value(cached).unwrap()["data"]["cached"],
                    true
                );
                assert_eq!(absent.calls.get(), 0);
            }
        }
    }
}

#[test]
fn accepted_write_has_one_attempt_and_cached_receipt_works_offline() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [61; 32]);
    let config = Config::default();
    let instance = instance();
    let backend = fake("ok");
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let result = write::execute(
        plan(),
        Some("send-key"),
        false,
        &session,
        &store,
        Some(&refs),
        deadline(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap()["data"]["receipt"]["messageId"],
        "9007199254740993"
    );
    assert_eq!(backend.writes.get(), 1);
    let backend = fake("offline");
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let result = serde_json::to_value(
        write::execute(
            plan(),
            Some("send-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(result["data"]["cached"], true);
    assert_eq!(backend.calls.get(), 0);
}

#[test]
fn attachment_writes_bind_content_to_pending_intent_and_do_not_replay_unknown_steps() {
    for mode in ["ok", "attachment_timeout"] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(temp.path().join("refs"), [91; 32]);
        let source = temp.path().join("fixture.bin");
        std::fs::write(&source, b"synthetic attachment bytes").unwrap();
        let backend = fake(mode);
        let config = Config::default();
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let make_plan = || {
            let mut plan = plan();
            plan.attachments = vec![
                write::Attachment {
                    kind: qq_cli::messaging::attachments::Kind::Image,
                    path: source.clone(),
                },
                write::Attachment {
                    kind: qq_cli::messaging::attachments::Kind::File,
                    path: source.clone(),
                },
            ];
            plan
        };
        let first = write::execute(
            make_plan(),
            Some("attachment-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline(),
        );
        if mode == "ok" {
            assert_eq!(
                serde_json::to_value(first.unwrap()).unwrap()["data"]["receipt"]["fileTransfer"],
                "not_observable"
            );
        } else {
            assert_eq!(first.err().unwrap().code, "WRITE_OUTCOME_UNKNOWN");
        }
        let calls = backend.calls.get();
        let _ = write::execute(
            make_plan(),
            Some("attachment-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline(),
        );
        assert_eq!(backend.calls.get(), calls);
        assert_eq!(backend.writes.get(), 1);
        let record = ledger::get(&store, "attachment-key").unwrap();
        assert_eq!(
            record.outcome,
            if mode == "ok" {
                Outcome::Accepted
            } else {
                Outcome::Unknown
            }
        );
        assert_eq!(
            record.receipt.as_ref().unwrap()["attachments"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let entries = std::fs::read_dir(store.root().join("attachments"))
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(entries.len(), 2);
        for entry in entries {
            let pointer: Value = serde_json::from_slice(
                &std::fs::read(entry.path().join(".qq-cli-attachment.json")).unwrap(),
            )
            .unwrap();
            let owned: Value = refs
                .get("a1", pointer["reference"].as_str().unwrap())
                .unwrap();
            assert_eq!(owned["intentPayloadHash"], record.payload_hash);
        }
        std::fs::write(&source, b"changed bytes").unwrap();
        assert_eq!(
            write::execute(
                make_plan(),
                Some("attachment-key"),
                false,
                &session,
                &store,
                Some(&refs),
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "REQUEST_CONFLICT"
        );
        assert_eq!(backend.writes.get(), 1);
    }
}

#[test]
fn attachment_only_dry_run_does_not_create_intent_or_retain_snapshots() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let source = temp.path().join("fixture.bin");
    std::fs::write(&source, b"synthetic attachment bytes").unwrap();
    let backend = fake("ok");
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut plan = plan();
    plan.segments.clear();
    plan.attachments = vec![write::Attachment {
        kind: qq_cli::messaging::attachments::Kind::File,
        path: source,
    }];
    let result = write::execute(plan, None, true, &session, &store, None, deadline()).unwrap();
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["data"]["segmentCount"], 1);
    let descriptor = qq_cli::cli::registry::get("message.send").unwrap();
    assert!(
        jsonschema::validator_for(&descriptor["responseSchema"])
            .unwrap()
            .is_valid(&result)
    );
    assert_eq!(backend.writes.get(), 0);
    assert!(!store.root().join("requests").exists());
    assert_eq!(
        std::fs::read_dir(store.root().join("attachments"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn corrupt_cached_acceptance_never_reaches_backend_or_becomes_a_fresh_write() {
    for mode in [
        "missing_receipt",
        "numeric_id",
        "wrong_target",
        "extra_field",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let config = Config::default();
        let instance = instance();
        let backend = fake("ok");
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        write::execute(
            plan(),
            Some("receipt-key"),
            false,
            &session,
            &store,
            None,
            deadline(),
        )
        .unwrap();
        let path = std::fs::read_dir(store.root().join("requests"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let mut record: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        match mode {
            "missing_receipt" => {
                record.as_object_mut().unwrap().remove("receipt");
            }
            "numeric_id" => record["receipt"]["messageId"] = json!(40001),
            "wrong_target" => record["receipt"]["conversation"] = json!("group:20002"),
            _ => record["receipt"]["private_url"] = json!("https://invalid.example/"),
        }
        std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        let offline = fake("offline");
        let session = Session {
            backend: &offline,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let error = write::execute(
            plan(),
            Some("receipt-key"),
            false,
            &session,
            &store,
            None,
            deadline(),
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "REQUEST_RECEIPT_INVALID");
        assert_eq!(offline.calls.get(), 0);
        assert_eq!(backend.writes.get(), 1);
    }
    assert_eq!(
        qq_cli::messaging::receipts::message_id(&json!(u64::MAX)).unwrap(),
        u64::MAX.to_string()
    );
    assert_eq!(
        qq_cli::messaging::receipts::message_id(&json!(i64::MIN)).unwrap(),
        i64::MIN.to_string()
    );
}

#[test]
fn uncertain_responses_and_account_switch_never_replay() {
    for mode in [
        "timeout",
        "business",
        "missing_id",
        "zero_id",
        "oversized_id",
        "changed",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(temp.path().join("refs"), [62; 32]);
        let config = Config::default();
        let instance = instance();
        let backend = fake(mode);
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        for _ in 0..2 {
            let error = write::execute(
                plan(),
                Some("unknown-key"),
                false,
                &session,
                &store,
                Some(&refs),
                deadline(),
            )
            .err()
            .unwrap();
            assert_eq!(error.code, "WRITE_OUTCOME_UNKNOWN");
            assert!(!error.retry_safe);
        }
        assert_eq!(backend.writes.get(), 1);
        assert_eq!(
            ledger::get(&store, "unknown-key").unwrap().outcome,
            Outcome::Unknown
        );
    }
}

#[test]
fn definitive_preflight_failure_and_auth_refusal_are_recorded_without_retry() {
    for mode in ["offline", "auth"] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(temp.path().join("refs"), [63; 32]);
        let config = Config::default();
        let instance = instance();
        let backend = fake(mode);
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let error = write::execute(
            plan(),
            Some("rejected-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline(),
        )
        .err()
        .unwrap();
        assert_eq!(
            error.outcome,
            Some(qq_cli::core::contracts::write::Outcome::Rejected)
        );
        assert_eq!(
            ledger::get(&store, "rejected-key").unwrap().outcome,
            Outcome::Rejected
        );
        assert_eq!(
            write::execute(
                plan(),
                Some("rejected-key"),
                false,
                &session,
                &store,
                Some(&refs),
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "WRITE_REJECTED"
        );
        assert_eq!(backend.writes.get(), usize::from(mode == "auth"));
    }
}

#[test]
fn dry_run_and_policy_denial_do_not_create_state_or_write() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [64; 32]);
    let mut config = Config::default();
    let instance = instance();
    let backend = fake("ok");
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert!(
        write::execute(
            plan(),
            None,
            true,
            &session,
            &store,
            Some(&refs),
            deadline()
        )
        .is_ok()
    );
    assert_eq!(backend.writes.get(), 0);
    assert!(!store.root().exists());
    config.permissions.send = false;
    let before = backend.calls.get();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert_eq!(
        write::execute(
            plan(),
            Some("denied-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(backend.calls.get(), before);
    assert!(!store.root().exists());
}

#[test]
fn independent_processes_and_a_terminated_writer_share_one_durable_intent() {
    use std::process::{Command, Stdio};
    if let Some(root) = std::env::var_os("QQ_CLI_TEST_REQUEST_ROOT") {
        let root = std::path::PathBuf::from(root);
        let store = Store::new(root.join("state"));
        let refs = References::with_key(root.join("refs"), [65; 32]);
        let config = Config::default();
        let instance = instance();
        let mode = if std::env::var_os("QQ_CLI_TEST_CRASH").is_some() {
            "crash"
        } else {
            "ok"
        };
        let backend = fake(mode);
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let result = write::execute(
            plan(),
            Some("process-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline(),
        );
        match result {
            Ok(_) => (),
            Err(error) => assert_eq!(error.code, "WRITE_OUTCOME_UNKNOWN"),
        };
        return;
    }
    let launch = |root: &std::path::Path, crash: bool| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "independent_processes_and_a_terminated_writer_share_one_durable_intent",
                "--test-threads=1",
            ])
            .env("QQ_CLI_TEST_REQUEST_ROOT", root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if crash {
            command.env("QQ_CLI_TEST_CRASH", "1");
        } else {
            command.env_remove("QQ_CLI_TEST_CRASH");
        }
        command.spawn().unwrap()
    };
    let wait = |mut child: std::process::Child| {
        let until = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= until {
                child.kill().unwrap();
                panic!("synthetic child deadline");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.wait_with_output().unwrap()
    };
    for crash in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        qq_cli::storage::state::private_dir(&temp.path().join("state")).unwrap();
        if crash {
            assert_eq!(wait(launch(temp.path(), true)).status.code(), Some(77));
            assert!(wait(launch(temp.path(), false)).status.success());
            assert_eq!(
                ledger::get(&Store::new(temp.path().join("state")), "process-key")
                    .unwrap()
                    .outcome,
                Outcome::Unknown
            );
        } else {
            let first = launch(temp.path(), false);
            let second = launch(temp.path(), false);
            for child in [first, second] {
                let output = wait(child);
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
        }
        assert_eq!(
            std::fs::read_to_string(temp.path().join("attempts.txt")).unwrap(),
            "attempt\n"
        );
    }
}

#[test]
fn reply_revalidates_its_target_only_for_a_new_attempt_and_cached_result_survives_restart() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [66; 32]);
    let config = Config::default();
    let mut instance = instance();
    let backend = fake("reply");
    let attachment = temp.path().join("reply.bin");
    std::fs::write(&attachment, b"synthetic attachment bytes").unwrap();
    let locator = qq_cli::messaging::message::Locator {
        scope: qq_cli::core::domain::Scope {
            instance_id: instance.id.clone(),
            account: "10001".into(),
        },
        run_id: instance.run_id.clone(),
        conversation: Conversation::Group("20001".into()),
        backend_id: "40001".into(),
        time_seconds: Some(1700000000),
        sequence: Some("101".into()),
        sender_id: Some("10002".into()),
    };
    let reply = || Plan {
        reply: Some(locator.clone()),
        attachments: vec![write::Attachment {
            kind: qq_cli::messaging::attachments::Kind::File,
            path: attachment.clone(),
        }],
        ..plan()
    };
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert!(
        write::execute(
            reply(),
            Some("reply-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline()
        )
        .is_ok()
    );
    assert_eq!(backend.writes.get(), 1);
    instance.run_id = "new-run".into();
    let backend = fake("offline");
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert!(
        write::execute(
            reply(),
            Some("reply-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline()
        )
        .is_ok()
    );
    assert_eq!(backend.calls.get(), 0);
    assert_eq!(
        write::execute(
            reply(),
            Some("new-reply-key"),
            false,
            &session,
            &store,
            Some(&refs),
            deadline()
        )
        .err()
        .unwrap()
        .outcome,
        Some(qq_cli::core::contracts::write::Outcome::Rejected)
    );
    assert_eq!(backend.writes.get(), 0);
}

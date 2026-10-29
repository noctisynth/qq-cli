use super::*;
use crate::{
    cli::args::{ApiCommand, Cli, Command},
    storage::state::Instance,
    transport::client::Action,
};
use clap::Parser;

#[test]
fn source_guard_is_inside_fresh_intent_and_raw_keys_cannot_discard_binding() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let references = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let config = crate::storage::config::Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    let args = args("delete_msg");
    let payload = json!({"message_id":"42"});
    let checks = Cell::new(0);
    for _ in 0..2 {
        let result = execute_guarded(
            &args,
            payload.clone(),
            &session,
            &store,
            Some(&references),
            deadline(),
            Some(Guard {
                receipt_target: None,
                targets: vec!["group:20001".parse().unwrap()],
                binding: json!({"syntheticLocator":"first"}),
                validate: || {
                    assert_eq!(
                        ledger::get(&store, "api-example")?.outcome,
                        Outcome::Pending
                    );
                    checks.set(checks.get() + 1);
                    Ok(())
                },
            }),
        )
        .unwrap();
        assert_eq!(result.exit_code(), 0);
    }
    assert_eq!(checks.get(), 1);
    assert_eq!(backend.writes.get(), 1);
    assert_eq!(
        execute_prepared(
            &args,
            payload.clone(),
            &session,
            &store,
            Some(&references),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "REQUEST_CONFLICT"
    );
    let result = execute_guarded(
        &args,
        payload,
        &session,
        &store,
        Some(&references),
        deadline(),
        Some(Guard {
            receipt_target: None,
            targets: vec!["group:20001".parse().unwrap()],
            binding: json!({"syntheticLocator":"second"}),
            validate: || panic!("conflicting binding must not validate"),
        }),
    );
    assert_eq!(result.err().unwrap().code, "REQUEST_CONFLICT");
    assert_eq!(backend.writes.get(), 1);
    let mut failed = args;
    failed.request_id = Some("source-rejected".parse().unwrap());
    for _ in 0..2 {
        let result = execute_guarded(
            &failed,
            json!({"message_id":"42"}),
            &session,
            &store,
            Some(&references),
            deadline(),
            Some(Guard {
                receipt_target: None,
                targets: vec!["group:20001".parse().unwrap()],
                binding: json!({"syntheticLocator":"first"}),
                validate: || {
                    checks.set(checks.get() + 1);
                    Err(AppError::new(
                        "STALE_REFERENCE",
                        "Synthetic stale source.",
                        4,
                    ))
                },
            }),
        );
        match result {
            Err(error) => assert_eq!(error.code, "STALE_REFERENCE"),
            Ok(result) => assert_ne!(result.exit_code(), 0),
        }
    }
    assert_eq!(checks.get(), 2);
    assert_eq!(
        ledger::get(&store, "source-rejected").unwrap().outcome,
        Outcome::Rejected
    );
    assert_eq!(backend.writes.get(), 1);
}

#[test]
fn raw_message_sources_need_read_permission_and_cannot_hide_behind_the_recipient() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let references = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let instance = instance();
    let mut config = crate::storage::config::Config::default();
    config.permissions.targets = vec!["group:20001".into()];
    for payload in [
        json!({"group_id":"20001","message":[{"type":"node","data":{"id":"42"}}]}),
        json!({"group_id":"20001","message":"[CQ:reply,id=42]"}),
    ] {
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "v4.18.38",
        };
        assert_eq!(
            execute_prepared(
                &args("send_group_msg"),
                payload,
                &session,
                &store,
                Some(&references),
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "API_TARGET_SCOPE_UNKNOWN"
        );
    }
    config.permissions.targets.clear();
    config.permissions.read = false;
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    assert_eq!(
        execute_prepared(
            &args("forward_group_single_msg"),
            json!({"group_id":"20001","message_id":"42"}),
            &session,
            &store,
            Some(&references),
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "PERMISSION_DENIED"
    );
    assert_eq!(backend.calls.get(), 0);
    assert_eq!(backend.writes.get(), 0);
}

#[test]
fn abandoned_pending_and_oversized_results_remain_unknown_without_replay() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let mut config = crate::storage::config::Config::default();
    let instance = instance();
    let args = args("future_example_action");
    let scope = Scope {
        instance_id: instance.id.clone(),
        account: "10001".into(),
    };
    let fingerprint = json!({"action":args.action.to_string(),"parameters":refs.api_fingerprint(&json!({})).unwrap()});
    drop(
        ledger::begin(
            &store,
            "api-example",
            "api.future_example_action",
            &scope,
            &fingerprint,
            deadline(),
        )
        .unwrap(),
    );
    {
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "v4.18.38",
        };
        let result = execute(&args, &session, &store, Some(&refs), deadline()).unwrap();
        assert_ne!(result.exit_code(), 0);
        assert_eq!(
            ledger::get(&store, "api-example").unwrap().outcome,
            Outcome::Unknown
        );
        assert_eq!(backend.calls.get(), 0);
        assert_eq!(backend.writes.get(), 0);
    }
    config.response_bytes = 1024;
    backend.mode.set("large");
    let mut second = args;
    second.request_id = Some("api-second".parse().unwrap());
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    assert_eq!(
        execute(&second, &session, &store, Some(&refs), deadline())
            .err()
            .unwrap()
            .code,
        "WRITE_OUTCOME_UNKNOWN"
    );
    assert_eq!(
        ledger::get(&store, "api-second").unwrap().outcome,
        Outcome::Unknown
    );
    assert_eq!(backend.writes.get(), 1);
}
use std::{cell::Cell, time::Duration};

struct Fake {
    calls: Cell<usize>,
    writes: Cell<usize>,
    mode: Cell<&'static str>,
}
impl Backend for Fake {
    fn call(&self, action: Action, _: &Value, _: Instant) -> Result<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.mode.get() == "offline" {
            return Err(AppError::protocol());
        }
        Ok(match action {
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::Status => json!({"online":true}),
            Action::LoginInfo => {
                json!({"user_id":if self.mode.get()=="changed" && self.writes.get()>0 {"10002"} else {"10001"}})
            }
            _ => panic!("Unexpected identity action"),
        })
    }
    fn call_raw(&self, _: &crate::core::input::ActionName, _: &Value, _: Instant) -> Result<Value> {
        self.writes.set(self.writes.get() + 1);
        match self.mode.get() {
            "large" => Ok(json!({"status":"ok","retcode":0,"data":"x".repeat(2048)})),
            "interrupted" => Err(AppError::protocol()),
            "rejected" => Ok(
                json!({"status":"failed","retcode":123,"data":null,"message":"synthetic-result-secret","extra":[false,null]}),
            ),
            "async" => Ok(json!({"status":"async","retcode":1,"data":null})),
            _ => Ok(
                json!({"status":"ok","retcode":0,"data":{"token":"synthetic-result-secret"},"extra":[false,null]}),
            ),
        }
    }
}
fn args(action: &str) -> ApiCallArgs {
    let cli = Cli::try_parse_from([
        "qq-cli",
        "api",
        "call",
        action,
        "--request-id",
        "api-example",
    ])
    .unwrap();
    let Command::Api {
        command: ApiCommand::Call(args),
    } = cli.command
    else {
        panic!()
    };
    args
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
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
#[test]
fn keyed_unknown_api_requests_are_once_only_and_never_store_inputs_or_results() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let config = crate::storage::config::Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    let file = temp.path().join("params.json");
    std::fs::write(
        &file,
        br#"{"secret":"synthetic-param-secret","b":null,"a":1}"#,
    )
    .unwrap();
    let mut args = args("future_example_action");
    args.params_file = Some(file.clone());
    let result = execute(&args, &session, &store, Some(&refs), deadline()).unwrap();
    let output = serde_json::to_value(result).unwrap();
    assert_eq!(output["data"]["result"]["extra"], json!([false, null]));
    assert_eq!(backend.writes.get(), 1);
    std::fs::write(
        &file,
        br#"{"a":1,"b":null,"secret":"synthetic-param-secret"}"#,
    )
    .unwrap();
    backend.mode.set("offline");
    let cached = execute(&args, &session, &store, Some(&refs), deadline()).unwrap();
    let cached = serde_json::to_value(cached).unwrap();
    assert_eq!(cached["data"]["deduplicated"], true);
    assert_eq!(cached["data"]["resultAvailable"], false);
    assert!(cached["data"].get("result").is_none());
    assert_eq!(backend.writes.get(), 1);
    let mut other = instance.clone();
    other.id = "another-instance".into();
    {
        let other_session = Session {
            instance: &other,
            ..session
        };
        let cached = execute(&args, &other_session, &store, Some(&refs), deadline()).unwrap();
        assert_eq!(
            serde_json::to_value(cached).unwrap()["meta"]["scope"]["instanceId"],
            "instance-example"
        );
    }
    other.account = Some("10002".into());
    {
        let other_session = Session {
            instance: &other,
            ..session
        };
        assert_eq!(
            execute(&args, &other_session, &store, Some(&refs), deadline())
                .err()
                .unwrap()
                .code,
            "REQUEST_CONFLICT"
        );
    }
    std::fs::write(&file, br#"{"secret":"changed"}"#).unwrap();
    assert_eq!(
        execute(&args, &session, &store, Some(&refs), deadline())
            .err()
            .unwrap()
            .code,
        "REQUEST_CONFLICT"
    );
    for file in std::fs::read_dir(store.root().join("requests")).unwrap() {
        let body = std::fs::read_to_string(file.unwrap().path()).unwrap();
        assert!(!body.contains("synthetic-param-secret"));
        assert!(!body.contains("synthetic-result-secret"));
    }
    let scope = Scope {
        instance_id: instance.id,
        account: "10001".into(),
    };
    assert!(
        ledger::begin(
            &store,
            "api-example",
            "send",
            &scope,
            &json!({}),
            deadline()
        )
        .is_err()
    );
}
#[test]
fn unknown_failures_and_identity_changes_never_replay_but_explicit_rejections_are_preserved() {
    for (mode, expected) in [
        ("interrupted", Outcome::Unknown),
        ("changed", Outcome::Unknown),
        ("async", Outcome::Unknown),
        ("rejected", Outcome::Rejected),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(store.root().join("references"), [7; 32]);
        let backend = Fake {
            calls: Cell::new(0),
            writes: Cell::new(0),
            mode: Cell::new(mode),
        };
        let config = crate::storage::config::Config::default();
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "v4.18.38",
        };
        let args = args("future_example_action");
        match execute(&args, &session, &store, Some(&refs), deadline()) {
            Ok(result) => {
                assert_ne!(result.exit_code(), 0);
                let output = serde_json::to_value(result).unwrap();
                if mode == "rejected" {
                    assert_eq!(output["data"]["result"]["retcode"], 123);
                }
            }
            Err(error) => assert_eq!(error.code, "WRITE_OUTCOME_UNKNOWN"),
        }
        assert_eq!(
            ledger::get(&store, "api-example").unwrap().outcome,
            expected
        );
        let _ = execute(&args, &session, &store, Some(&refs), deadline());
        assert_eq!(backend.writes.get(), 1);
    }
}
#[test]
fn target_restrictions_and_send_policy_reject_before_business_requests() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let mut config = crate::storage::config::Config::default();
    config.permissions.targets = vec!["group:20001".into()];
    let instance = instance();
    {
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "v4.18.38",
        };
        assert_eq!(
            execute(
                &args("future_example_action"),
                &session,
                &store,
                Some(&refs),
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "API_TARGET_SCOPE_UNKNOWN"
        );
    }
    config.permissions.targets.clear();
    config.permissions.send = false;
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    assert_eq!(
        execute(
            &args("future_example_action"),
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
    assert_eq!(backend.calls.get(), 0);
    assert_eq!(backend.writes.get(), 0);
}

#[test]
fn poke_targets_follow_actual_alias_payload_and_reject_rounding_or_misdirection() {
    for action in ["friend_poke", "group_poke", "send_poke"] {
        let group = json!({"group_id":"20001","user_id":"10002","target_id":"10003"});
        assert_eq!(
            target(action, &group, true).unwrap().unwrap().to_string(),
            "group:20001"
        );
        let private = json!({"user_id":"10002"});
        assert_eq!(
            target(action, &private, true).unwrap().unwrap().to_string(),
            "private:10002"
        );
        for bad in [
            json!({"user_id":"10002","target_id":"10003"}),
            json!({"user_id":"10002","group_id":20001}),
            json!({"user_id":"10002","target_id":10003}),
            json!({"user_id":"9007199254740993"}),
            json!({"user_id":"10002","group_id":"9007199254740993"}),
        ] {
            assert!(target(action, &bad, true).is_err());
        }
    }
    for action in [
        "set_msg_emoji_like",
        "fetch_emoji_like",
        "get_emoji_likes",
        "fetch_ptt_text",
    ] {
        assert!(target(action, &json!({"message_id":"42","group_id":"20001"}), true).is_err());
        assert!(reads_message_source(action, &json!({})));
    }
}

#[test]
fn card_aliases_follow_group_precedence_and_quick_ack_is_never_completion() {
    for action in ["ArkSharePeer", "send_ark_share"] {
        assert_eq!(
            target(action, &json!({"user_id":"10002","group_id":"20001"}), true)
                .unwrap()
                .unwrap()
                .to_string(),
            "group:20001"
        );
        assert_eq!(
            target(action, &json!({"user_id":"10002","group_id":""}), true)
                .unwrap()
                .unwrap()
                .to_string(),
            "private:10002"
        );
        assert!(target(action, &json!({"user_id":"10002","group_id":20001}), true).is_err());
        assert_eq!(
            status(
                action,
                &json!({"status":"ok","retcode":0,"data":{"result":17}})
            )
            .unwrap()
            .1,
            17
        );
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [7; 32]);
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let config = crate::storage::config::Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    let args = args(".handle_quick_operation");
    for i in 0..2 {
        let envelope = execute_prepared(
            &args,
            json!({"context":{},"operation":{}}),
            &session,
            &store,
            Some(&refs),
            deadline(),
        )
        .unwrap();
        assert_eq!(envelope.exit_code(), 5);
        let wire = serde_json::to_value(envelope).unwrap();
        assert_eq!(wire["data"]["outcome"], "unknown");
        assert_eq!(wire["data"]["deduplicated"], i == 1);
    }
    assert_eq!(backend.writes.get(), 1);
    assert_eq!(
        ledger::get(&store, "api-example").unwrap().outcome,
        Outcome::Unknown
    );
    assert!(
        target(
            ".handle_quick_operation",
            &json!({"context":{"group_id":"20001"}}),
            true
        )
        .is_err()
    );
}

#[test]
fn generated_cards_and_mini_app_queries_need_no_send_permission_or_write_key() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let backend = Fake {
        calls: Cell::new(0),
        writes: Cell::new(0),
        mode: Cell::new("ok"),
    };
    let mut config = crate::storage::config::Config::default();
    config.permissions.send = false;
    config.permissions.targets = vec!["group:20001".into()];
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    for (action, payload) in [
        (
            "ArkSharePeer",
            json!({"user_id":"10002","group_id":"20001","phone_number":""}),
        ),
        (
            "send_ark_share",
            json!({"user_id":"10002","group_id":"20001","phone_number":""}),
        ),
        ("ArkShareGroup", json!({"group_id":"20001"})),
        ("send_group_ark_share", json!({"group_id":"20001"})),
        ("get_mini_app_ark", json!({"type":"bili"})),
    ] {
        let mut args = args(action);
        args.request_id = None;
        let reply = execute_prepared(&args, payload, &session, &store, None, deadline()).unwrap();
        assert_eq!(reply.exit_code(), 0);
    }
    assert_eq!(backend.writes.get(), 5);
    assert!(ledger::get(&store, "api-example").is_err());
}

#[test]
fn group_message_tasks_honor_native_sequence_precedence_and_numeric_targets() {
    for action in ["set_group_todo", "complete_group_todo", "cancel_group_todo"] {
        let seq = json!({"group_id":"20001","message_seq":"7","message_id":"42"});
        assert!(!reads_message_source(action, &seq));
        assert_eq!(
            target(action, &seq, true).unwrap().unwrap().to_string(),
            "group:20001"
        );
        let fallback = json!({"group_id":"20001","message_id":"42"});
        assert!(reads_message_source(action, &fallback));
        assert!(target(action, &fallback, true).is_err());
        assert!(
            target(
                action,
                &json!({"group_id":"9007199254740993","message_seq":"7"}),
                true
            )
            .is_err()
        );
    }
    let tuple = json!({"group_id":"20001","msg_seq":"7","msg_random":"9","message_id":"42"});
    assert!(!reads_message_source("delete_essence_msg", &tuple));
    assert_eq!(
        target("delete_essence_msg", &tuple, true)
            .unwrap()
            .unwrap()
            .to_string(),
        "group:20001"
    );
    assert!(
        target(
            "delete_essence_msg",
            &json!({"group_id":"20001","message_id":"42"}),
            true
        )
        .is_err()
    );
    for action in ["set_group_sign", "send_group_sign"] {
        assert!(target(action, &json!({"group_id":"9007199254740993"}), true).is_err());
        assert_eq!(
            target(action, &json!({"group_id":"20001"}), true)
                .unwrap()
                .unwrap()
                .to_string(),
            "group:20001"
        );
    }
}

#[test]
fn own_repository_operations_do_not_accept_a_claimed_favorite_conversation() {
    assert!(
        target(
            "get_collection_list",
            &json!({"category":"0","count":"1","group_id":"20001"}),
            true
        )
        .is_err()
    );
    for action in [
        "create_collection",
        "fetch_custom_face",
        "fetch_custom_face_detail",
        "delete_custom_face",
        "set_custom_face_desc",
    ] {
        assert!(target(action, &json!({}), true).unwrap().is_none());
    }
}

#[test]
fn file_calls_bind_content_retain_before_native_access_and_pin_authenticated_sources() {
    use crate::messaging::attachments::{self, Kind};
    use std::{cell::RefCell, path::PathBuf, time::SystemTime};
    struct Files<'a> {
        identity: Fake,
        store: &'a Store,
        refs: &'a References,
        path: RefCell<Option<PathBuf>>,
    }
    impl Backend for Files<'_> {
        fn call(&self, action: Action, payload: &Value, deadline: Instant) -> Result<Value> {
            self.identity.call(action, payload, deadline)
        }
        fn call_raw(
            &self,
            action: &crate::core::input::ActionName,
            payload: &Value,
            deadline: Instant,
        ) -> Result<Value> {
            assert_eq!(&**action, "add_custom_face");
            let path = PathBuf::from(payload["file"].as_str().unwrap());
            assert!(
                path.canonicalize().unwrap().starts_with(
                    self.store
                        .root()
                        .join("attachments")
                        .canonicalize()
                        .unwrap()
                )
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"synthetic-source-bytes");
            let pointer: Value = serde_json::from_slice(
                &std::fs::read(path.parent().unwrap().join(".qq-cli-attachment.json")).unwrap(),
            )
            .unwrap();
            let owned: Value = self
                .refs
                .get("a1", pointer["reference"].as_str().unwrap())?;
            assert_eq!(owned["persistent"], true);
            let record = ledger::get(self.store, "api-example")?;
            assert_eq!(record.outcome, Outcome::Pending);
            assert_eq!(owned["intentPayloadHash"], record.payload_hash);
            self.path.replace(Some(path));
            self.identity.call_raw(action, payload, deadline)
        }
    }
    for mode in ["ok", "rejected", "interrupted", "changed"] {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(store.root().join("references"), [7; 32]);
        let source = temp.path().join("image.bin");
        std::fs::write(&source, b"synthetic-source-bytes").unwrap();
        let backend = Files {
            identity: Fake {
                calls: Cell::new(0),
                writes: Cell::new(0),
                mode: Cell::new(mode),
            },
            store: &store,
            refs: &refs,
            path: RefCell::new(None),
        };
        let config = crate::storage::config::Config::default();
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "v4.18.38",
        };
        let args = args("add_custom_face");
        for _ in 0..2 {
            let snapshot = attachments::prepare(&source, Kind::Image, &store, deadline()).unwrap();
            let payload = json!({"file":snapshot.path()});
            let reply = execute_file(
                &args,
                (payload, snapshot),
                &session,
                &store,
                Some(&refs),
                deadline(),
            );
            let code = reply.map(|v| v.exit_code()).unwrap_or_else(|e| e.exit_code);
            assert_eq!(
                code,
                match mode {
                    "ok" => 0,
                    "rejected" => 4,
                    _ => 5,
                },
                "{mode}"
            );
        }
        assert_eq!(backend.identity.writes.get(), 1);
        let path = backend.path.borrow().clone().unwrap();
        assert!(path.exists());
        assert_eq!(
            std::fs::read_dir(store.root().join("attachments"))
                .unwrap()
                .count(),
            1
        );
        let raw = execute_prepared(
            &args,
            json!({"file":path}),
            &session,
            &store,
            Some(&refs),
            deadline(),
        );
        assert_eq!(raw.err().unwrap().code, "REQUEST_CONFLICT");
        std::fs::write(&source, b"different-source-bytes").unwrap();
        let changed = attachments::prepare(&source, Kind::Image, &store, deadline()).unwrap();
        let payload = json!({"file":changed.path()});
        assert_eq!(
            execute_file(
                &args,
                (payload, changed),
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
        assert_eq!(backend.identity.writes.get(), 1);
        let old = SystemTime::now() + Duration::from_secs(2 * 24 * 60 * 60);
        let pruned = attachments::cache::prune(&store, &refs, None, old, deadline()).unwrap();
        assert_eq!(
            serde_json::to_value(pruned).unwrap()["data"]["removedFiles"],
            0
        );
        assert!(path.exists());
        let marker: Value = serde_json::from_slice(
            &std::fs::read(path.parent().unwrap().join(".qq-cli-attachment.json")).unwrap(),
        )
        .unwrap();
        let token = marker["reference"].as_str().unwrap();
        let prefix = token.rsplit_once('.').unwrap().0;
        let file = store
            .root()
            .join("references")
            .join(format!("{prefix}.json"));
        let mut owned: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        owned["persistent"] = json!(false);
        std::fs::write(&file, serde_json::to_vec(&owned).unwrap()).unwrap();
        assert!(refs.get::<Value>("a1", token).is_err());
        let preserved = attachments::cache::prune(&store, &refs, None, old, deadline()).unwrap();
        let preserved = serde_json::to_value(preserved).unwrap();
        assert_eq!(preserved["data"]["removedFiles"], 0);
        assert_eq!(preserved["data"]["preservedEntries"], 1);
        assert!(path.exists());
    }
}

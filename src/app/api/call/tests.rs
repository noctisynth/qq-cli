use super::*;
use crate::{
    cli::args::{ApiCommand, Cli, Command},
    storage::state::Instance,
    transport::client::Action,
};
use clap::Parser;

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

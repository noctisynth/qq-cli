#[path = "common/group_requests.rs"]
mod cases;
use serde_json::{Value, json};
use std::process::Command;
fn run(args: &[String], config: &std::path::Path) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(out.stderr.is_empty());
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}
#[test]
fn group_request_protocols_and_decision_previews_are_offline() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("missing.json");
    for case in cases::cases() {
        let desc = qq_cli::transport::catalog::describe(&case.action.parse().unwrap());
        assert!(desc.parameters_known);
        let schema = serde_json::to_value(desc.parameter_schema.unwrap()).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&case.payload)
        );
        if case.write {
            let mut args = case.args.clone();
            args.push("--dry-run".into());
            let (code, result) = run(&args, &config);
            assert_eq!(code, 0, "{result}");
            assert!(!result.to_string().contains("synthetic-exact-flag"));
            assert_eq!(result["data"]["identityVerified"], false);
            let schema = qq_cli::cli::registry::get("group.respond").unwrap();
            assert!(
                jsonschema::validator_for(&schema["responseSchema"])
                    .unwrap()
                    .is_valid(&result)
            );
            assert_eq!(run(&case.args, &config).0, 2);
        }
    }
    let desc = qq_cli::transport::catalog::describe(&"set_group_add_request".parse().unwrap());
    let schema = serde_json::to_value(desc.parameter_schema.unwrap()).unwrap();
    assert!(
        jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&json!({"flag":"synthetic","reason":null}))
    );
    for args in [
        vec!["group", "respond", "flag", "--dry-run"],
        vec!["group", "requests", "--source", "invented"],
        vec![
            "group",
            "respond",
            "flag",
            "--approve",
            "true",
            "--search-limit",
            "0",
            "--dry-run",
        ],
    ] {
        assert_eq!(
            run(
                &args
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                &config
            )
            .0,
            2
        );
    }
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_group_requests_filter_deduplicate_aliases_and_refuse_imprecise_flags() {
    use qq_cli::storage::{
        credentials::{Credentials, SystemCredentials},
        state::{Installation, Instance, Store},
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    struct Cleanup(SystemCredentials);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete("synthetic-group-requests");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
    creds
        .0
        .set("synthetic-group-requests", "synthetic-cli-token")
        .unwrap();
    let cases = cases::cases();
    let expected = std::sync::Arc::new(std::sync::Mutex::new(
        cases
            .iter()
            .map(|c| (c.action, c.payload.clone(), c.data.clone(), c.rejected))
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let server_expected = expected.clone();
    let (endpoint, server) = http::serve(cases.len() * 7, move |action, payload| {
        let mut rejected = false;
        let data = match action {
            "get_status" => json!({"online":true}),
            "get_login_info" => json!({"user_id":"10001"}),
            "get_version_info" => json!({"app_version":"4.18.38"}),
            _ => {
                let (a, p, d, r) = server_expected.lock().unwrap().pop_front().unwrap();
                assert_eq!(action, a);
                assert_eq!(payload, &p);
                rejected = r;
                d
            }
        };
        serde_json::to_vec(&json!({"status":if rejected{"failed"}else{"ok"},"retcode":if rejected{1400}else{0},"data":data,"message":"synthetic-private-error"})).unwrap()
    });
    let mut tx = store
        .transaction(Instant::now() + Duration::from_secs(5))
        .unwrap();
    tx.registry.installations.push(Installation {
        id: "install-example".into(),
        version: "v4.18.38".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("unused"),
    });
    tx.registry.instances.push(Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint,
        ws_endpoint: None,
        credential_key: "synthetic-group-requests".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    });
    tx.commit().unwrap();
    let _managed = managed::Managed::attach(
        &store,
        &creds.0,
        "instance-example",
        "managed_group_requests_filter_deduplicate_aliases_and_refuse_imprecise_flags",
    );
    let server = server.start();
    let config = temp.path().join("config.json");
    let settings = |restricted: bool, scan: usize| {
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"scanLimit":scan,"permissions":{"targets":if restricted{vec!["group:20001"]}else{vec![]}}})).unwrap()).unwrap();
    };
    for (index, case) in cases.into_iter().enumerate() {
        settings(!case.write, case.scan);
        let key = format!("group-request-{index}");
        let mut args = case.args;
        if case.write {
            args.extend(["--request-id".into(), key.clone()]);
        }
        let (code, result) = run(&args, &config);
        assert_eq!(
            code,
            if case.rejected || case.scan == 1 {
                4
            } else {
                0
            },
            "{result}"
        );
        assert!(!result.to_string().contains("synthetic-private-error"));
        assert!(!result.to_string().contains("synthetic-hidden-field"));
        let schema = qq_cli::cli::registry::get(&format!("group.{}", case.leaf)).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{result}"
        );
        match index {
            0 => {
                assert_eq!(result["data"]["requests"].as_array().unwrap().len(), 1);
                assert_eq!(result["data"]["requests"][0]["requestFlag"], "1001");
            }
            1 => {
                assert_eq!(result["data"]["requests"].as_array().unwrap().len(), 1);
                assert_eq!(result["data"]["truncated"], true);
            }
            2 => {
                assert_eq!(result["data"]["requests"].as_array().unwrap().len(), 1);
                assert_eq!(result["data"]["requests"][0]["requestIdExact"], false);
                assert!(result["data"]["requests"][0].get("requestFlag").is_none());
            }
            _ => (),
        }
        if case.write {
            let (code, cached) = run(&args, &config);
            assert_eq!(code, if case.rejected { 4 } else { 0 });
            if case.rejected {
                assert_eq!(cached["error"]["outcome"], "rejected");
            } else {
                assert_eq!(cached["data"]["deduplicated"], true);
            }
            let raw = temp.path().join("raw.json");
            std::fs::write(&raw, serde_json::to_vec(&case.payload).unwrap()).unwrap();
            let args = vec![
                "api".into(),
                "call".into(),
                case.action.into(),
                "--params-file".into(),
                raw.to_string_lossy().into_owned(),
                "--request-id".into(),
                key,
            ];
            assert_eq!(run(&args, &config).0, if case.rejected { 4 } else { 0 });
        }
    }
    server.join().unwrap();
    assert!(expected.lock().unwrap().is_empty());
    settings(true, 1000);
    for (args, error) in [
        (
            vec![
                "group",
                "respond",
                "flag",
                "--approve",
                "true",
                "--request-id",
                "denied-approval",
            ],
            "API_TARGET_SCOPE_UNKNOWN",
        ),
        (
            vec!["group", "requests", "--group-id", "20002"],
            "TARGET_DENIED",
        ),
    ] {
        let (code, result) = run(
            &args
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            &config,
        );
        assert_eq!(code, 4);
        assert_eq!(result["error"]["code"], error);
    }
}

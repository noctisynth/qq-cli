#[path = "common/group_queries.rs"]
mod cases;
use serde_json::{Value, json};
use std::process::Command;
fn run(args: &[String], config: &std::path::Path) -> (i32, Value) {
    let o = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(o.stderr.is_empty());
    (
        o.status.code().unwrap(),
        serde_json::from_slice(&o.stdout).unwrap(),
    )
}
#[test]
fn group_query_schemas_come_from_the_reviewed_payloads() {
    for case in cases::cases() {
        let description = qq_cli::transport::catalog::describe(&case.action.parse().unwrap());
        assert!(description.parameters_known);
        assert_eq!(
            description.effect,
            qq_cli::core::contracts::api::Effect::Read
        );
        let schema = serde_json::to_value(description.parameter_schema.unwrap()).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&case.payload),
            "{}",
            case.action
        );
        let schema = qq_cli::cli::registry::get(case.path).unwrap();
        assert_eq!(schema["sideEffects"], "read");
        assert_eq!(schema["requestId"], "not required");
    }
    let temp = tempfile::tempdir().unwrap();
    for args in [
        vec!["group", "member", "20001"],
        vec!["group", "honors", "20001", "--kind", "invented"],
        vec!["group", "share", "20001", "--source-id", "-1"],
    ] {
        assert_eq!(
            run(
                &args
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                &temp.path().join("missing.json")
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
fn managed_group_queries_validate_scope_project_fields_and_retain_empty_ambiguity() {
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
            let _ = self.0.delete("synthetic-group-query");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
    creds
        .0
        .set("synthetic-group-query", "synthetic-cli-token")
        .unwrap();
    let cases = cases::cases();
    let expected = std::sync::Arc::new(std::sync::Mutex::new(
        cases
            .iter()
            .map(|c| (c.action, c.payload.clone(), c.data.clone()))
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let server_expected = expected.clone();
    let (endpoint, server) = http::serve(cases.len() * 7, move |action, payload| {
        let data = match action {
            "get_status" => json!({"online":true}),
            "get_login_info" => json!({"user_id":"10001"}),
            "get_version_info" => json!({"app_version":"4.18.38"}),
            _ => {
                let (a, p, d) = server_expected.lock().unwrap().pop_front().unwrap();
                assert_eq!(action, a);
                assert_eq!(payload, &p);
                d
            }
        };
        serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
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
        credential_key: "synthetic-group-query".into(),
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
        "managed_group_queries_validate_scope_project_fields_and_retain_empty_ambiguity",
    );
    let server = server.start();
    let config = temp.path().join("config.json");
    for case in cases {
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"scanLimit":case.scan_limit,"permissions":{"targets":["group:20001"]}})).unwrap()).unwrap();
        let (code, result) = run(&case.args, &config);
        assert_eq!(
            code,
            if case.error == Some("BACKEND_PROTOCOL") {
                3
            } else if case.error.is_some() {
                4
            } else {
                0
            },
            "{result}"
        );
        assert!(!result.to_string().contains("synthetic-hidden-field"));
        let contract = qq_cli::cli::registry::get(case.path).unwrap();
        assert!(
            jsonschema::validator_for(&contract["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{}: {result}",
            case.path
        );
        if let Some(error) = case.error {
            assert_eq!(result["error"]["code"], error);
            continue;
        }
        assert_eq!(result["meta"]["scope"]["account"], "10001");
        match case.path {
            "group.detail" => {
                assert_eq!(result["data"]["wholeGroupMuted"], true);
                assert!(result["data"].get("remark").is_none());
            }
            "group.honors" | "group.muted" => assert_eq!(result["data"]["complete"], false),
            "group.member" => assert_eq!(result["data"]["userId"], "10002"),
            "group.extended" => assert_eq!(result["data"]["owner"]["userId"], "10002"),
            _ => (),
        }
    }
    server.join().unwrap();
    assert!(expected.lock().unwrap().is_empty());
    for leaf in [
        "member",
        "detail",
        "extended",
        "share",
        "honors",
        "muted",
        "mention-quota",
    ] {
        let mut args = vec!["group".into(), leaf.into(), "20002".into()];
        if leaf == "member" {
            args.push("10002".into());
        }
        let (code, result) = run(&args, &config);
        assert_eq!(code, 4);
        assert_eq!(result["error"]["code"], "TARGET_DENIED");
    }
}

#[path = "common/contact_cases.rs"]
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
fn contact_contracts_and_write_previews_are_offline() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.json");
    for case in cases::cases() {
        let description = qq_cli::transport::catalog::describe(&case.action.parse().unwrap());
        assert!(description.parameters_known);
        let schema = serde_json::to_value(description.parameter_schema.unwrap()).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&case.payload)
        );
        if !case.write {
            continue;
        }
        let mut args = case.args.clone();
        args.push("--dry-run".into());
        let (code, result) = run(&args, &missing);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["data"]["identityVerified"], false);
        assert!(!result.to_string().contains("u_exact_flag"));
        assert!(!result.to_string().contains("req_exact_flag"));
        let contract = qq_cli::cli::registry::get(case.path).unwrap();
        assert!(
            jsonschema::validator_for(&contract["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
        assert_eq!(run(&case.args, &missing).0, 2);
    }
    for args in [
        vec!["contact", "respond", "flag", "--dry-run"],
        vec![
            "contact",
            "respond",
            "flag",
            "--approve",
            "invalid",
            "--dry-run",
        ],
        vec![
            "contact",
            "accept-doubt",
            "flag",
            "--approve",
            "false",
            "--dry-run",
        ],
        vec!["contact", "requests", "--limit", "0"],
    ] {
        assert_eq!(
            run(
                &args
                    .into_iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
                &missing
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
fn managed_contact_tasks_filter_identity_and_keep_native_rejections_durable() {
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
            let _ = self.0.delete("synthetic-contact");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
    creds
        .0
        .set("synthetic-contact", "synthetic-cli-token")
        .unwrap();
    let cases = cases::cases();
    let mut expected = cases
        .iter()
        .map(|c| (c.action, c.payload.clone(), c.data.clone()))
        .collect::<std::collections::VecDeque<_>>();
    expected.push_back((
        "get_stranger_info",
        json!({"user_id":"10002","no_cache":false}),
        json!({"user_id":"10003"}),
    ));
    expected.push_back(("get_friends_with_category",json!({}),json!([{ "categoryId":1,"categoryName":"Synthetic","categoryMbCount":1,"buddyList":[{"user_id":"10002"}]}])));
    let expected = std::sync::Arc::new(std::sync::Mutex::new(expected));
    let server_expected = expected.clone();
    let (endpoint, server) = http::serve(13 * 7, move |action, payload| {
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
        credential_key: "synthetic-contact".into(),
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
        "managed_contact_tasks_filter_identity_and_keep_native_rejections_durable",
    );
    let server = server.start();
    let config = temp.path().join("config.json");
    let settings = |restricted: bool, scan: usize| {
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"scanLimit":scan,"permissions":{"targets":if restricted{vec!["private:10002"]}else{vec![]}}})).unwrap()).unwrap();
    };
    settings(true, 1000);
    for (index, case) in cases.into_iter().enumerate() {
        if index == 6 {
            settings(false, 1000);
        }
        let key = format!("contact-{index}");
        let mut args = case.args;
        if case.write {
            args.extend(["--request-id".into(), key.clone()]);
        }
        let (code, result) = run(&args, &config);
        assert_eq!(code, if case.rejected { 4 } else { 0 }, "{result}");
        assert!(!result.to_string().contains("synthetic-private-error"));
        assert!(!result.to_string().contains("synthetic-hidden-field"));
        let contract = qq_cli::cli::registry::get(case.path).unwrap();
        assert!(
            jsonschema::validator_for(&contract["responseSchema"])
                .unwrap()
                .is_valid(&result),
            "{result}"
        );
        match case.path {
            "contact.categories" => assert_eq!(
                result["data"]["categories"][0]["friends"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            ),
            "contact.unilateral" => {
                assert_eq!(result["data"]["users"].as_array().unwrap().len(), 1)
            }
            "contact.requests" => {
                assert_eq!(result["data"]["requests"].as_array().unwrap().len(), 1)
            }
            _ => (),
        }
        if case.write {
            let (code, cached) = run(&args, &config);
            assert_eq!(code, if case.rejected { 4 } else { 0 }, "{cached}");
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
    settings(true, 1000);
    let args = vec!["contact", "profile", "10002"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let (code, result) = run(&args, &config);
    assert_ne!(code, 0);
    assert_eq!(result["error"]["code"], "BACKEND_PROTOCOL");
    settings(true, 1);
    let args = vec!["contact", "categories"]
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let (code, result) = run(&args, &config);
    assert_eq!(code, 4);
    assert_eq!(result["error"]["code"], "SCAN_LIMIT");
    server.join().unwrap();
    assert!(expected.lock().unwrap().is_empty());
    for args in [
        vec!["contact", "profile", "10003"],
        vec![
            "contact",
            "remove",
            "10003",
            "--request-id",
            "denied-remove",
        ],
        vec![
            "contact",
            "respond",
            "flag",
            "--approve",
            "true",
            "--request-id",
            "denied-approval",
        ],
    ] {
        let (code, result) = run(
            &args
                .into_iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            &config,
        );
        assert_eq!(code, 4);
        assert!(matches!(
            result["error"]["code"].as_str(),
            Some("TARGET_DENIED" | "API_TARGET_SCOPE_UNKNOWN")
        ));
    }
}

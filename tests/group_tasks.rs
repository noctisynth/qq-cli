#[path = "common/group_cases.rs"]
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
fn group_previews_clear_fields_and_require_explicit_dangerous_arguments() {
    let temp = tempfile::tempdir().unwrap();
    let avatar = temp.path().join("avatar.png");
    std::fs::write(&avatar, b"synthetic-image").unwrap();
    let config = temp.path().join("missing.json");
    for case in cases::cases(&avatar) {
        let mut args = case.args.clone();
        args.push("--dry-run".into());
        let (code, result) = run(&args, &config);
        assert_eq!(code, 0, "{args:?}: {result}");
        assert_eq!(result["data"]["operation"], format!("group.{}", case.leaf));
        assert_eq!(result["data"]["identityVerified"], false);
        let encoded = result.to_string();
        assert!(!encoded.contains("Synthetic"));
        assert!(!encoded.contains("base64"));
        let contract = qq_cli::cli::registry::get(&format!("group.{}", case.leaf)).unwrap();
        assert!(
            jsonschema::validator_for(&contract["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
        let desc = qq_cli::transport::catalog::describe(&case.action.parse().unwrap());
        assert!(desc.parameters_known);
        let schema = serde_json::to_value(desc.parameter_schema.unwrap()).unwrap();
        assert!(
            jsonschema::validator_for(&schema)
                .unwrap()
                .is_valid(&case.payload),
            "{}",
            case.action
        );
        let (code, _) = run(&case.args, &config);
        assert_eq!(code, 2, "key missing for {}", case.leaf);
    }
    for args in [
        vec!["group", "ban", "20001", "10002", "--dry-run"],
        vec!["group", "admin", "20001", "10002", "--dry-run"],
        vec!["group", "ban-all", "20001", "--dry-run"],
        vec!["group", "member-permissions", "20001", "--dry-run"],
        vec!["group", "robot-policy", "20001", "--dry-run"],
        vec!["group", "search-policy", "20001", "--dry-run"],
        vec!["group", "history-visibility", "20001", "--dry-run"],
        vec!["group", "invite-policy", "20001", "invented", "--dry-run"],
    ] {
        let (code, _) = run(
            &args.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
            &config,
        );
        assert_eq!(code, 2, "{args:?}");
    }
    let mut args = vec![
        "group".to_owned(),
        "kick-many".into(),
        "20001".into(),
        "--user-id".into(),
    ];
    args.extend((1..=101).map(|n| n.to_string()));
    args.push("--dry-run".into());
    assert_eq!(run(&args, &config).0, 2);
    let schema = qq_cli::cli::registry::get("group.member-permissions").unwrap();
    assert!(
        jsonschema::validator_for(&schema["inputSchema"])
            .unwrap()
            .is_valid(&json!({"group_id":"20001","upload_album":false,"dry_run":true}))
    );
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_group_tasks_scope_payload_and_deduplicate() {
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
            let _ = self.0.delete("synthetic-group");
            let _ = self.0.delete("reference-signing-key");
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
    creds
        .0
        .set("synthetic-group", "synthetic-cli-token")
        .unwrap();
    let avatar = temp.path().join("avatar.png");
    std::fs::write(&avatar, b"synthetic-image").unwrap();
    let cases = cases::cases(&avatar);
    let expected = std::sync::Arc::new(std::sync::Mutex::new(
        cases
            .iter()
            .map(|c| (c.action, c.payload.clone()))
            .collect::<std::collections::VecDeque<_>>(),
    ));
    let server_expected = expected.clone();
    let (endpoint, server) = http::serve(18 * 7, move |action, payload| {
        let data = match action {
            "get_status" => json!({"online":true}),
            "get_login_info" => json!({"user_id":"10001"}),
            "get_version_info" => json!({"app_version":"4.18.38"}),
            _ => {
                let (a, p) = server_expected.lock().unwrap().pop_front().unwrap();
                assert_eq!(action, a);
                let desc = qq_cli::transport::catalog::describe(&action.parse().unwrap());
                let schema = serde_json::to_value(desc.parameter_schema.unwrap()).unwrap();
                assert!(
                    jsonschema::validator_for(&schema)
                        .unwrap()
                        .is_valid(payload)
                );
                for (key, value) in p.as_object().unwrap() {
                    if value.is_number() {
                        assert_eq!(payload[key].as_f64(), value.as_f64());
                    } else {
                        assert_eq!(&payload[key], value);
                    }
                }
                assert_eq!(
                    payload.as_object().unwrap().len(),
                    p.as_object().unwrap().len()
                );
                Value::Null
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
        credential_key: "synthetic-group".into(),
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
        "managed_group_tasks_scope_payload_and_deduplicate",
    );
    let server = server.start();
    let config = temp.path().join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(
            &json!({"stateDir":store.root(),"permissions":{"targets":["group:20001"]}}),
        )
        .unwrap(),
    )
    .unwrap();
    for case in cases {
        let key = format!("group-{}", case.leaf);
        let mut args = case.args;
        args.extend(["--request-id".into(), key.clone()]);
        let (code, result) = run(&args, &config);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["data"]["groupId"], "20001");
        assert_eq!(result["data"]["outcome"], "accepted");
        let contract = qq_cli::cli::registry::get(&format!("group.{}", case.leaf)).unwrap();
        assert!(
            jsonschema::validator_for(&contract["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
        let (code, result) = run(&args, &config);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["data"]["deduplicated"], true);
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
        let (code, result) = run(&args, &config);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["data"]["deduplicated"], true);
    }
    server.join().unwrap();
    assert!(expected.lock().unwrap().is_empty());
    let args = vec![
        "group",
        "rename",
        "20002",
        "Synthetic",
        "--request-id",
        "denied-example",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect::<Vec<_>>();
    let (code, result) = run(&args, &config);
    assert_eq!(code, 4);
    assert_eq!(result["error"]["code"], "TARGET_DENIED");
}

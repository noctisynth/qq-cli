#![cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[path = "common/managed.rs"]
mod managed;
use qq_cli::storage::{
    credentials::{Credentials, SystemCredentials},
    state::{Installation, Instance, Store},
};
use serde_json::{Value, json};
use std::{
    process::Command,
    time::{Duration, Instant},
};
fn run(config: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .args(["--config", config.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
#[test]
fn owned_api_cli_calls_unknown_actions_and_queries_secret_free_receipts_without_replay() {
    if managed::child() {
        return;
    }
    for rejected in [false, true] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-api");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
        credentials
            .0
            .set("synthetic-api", "synthetic-cli-token")
            .unwrap();
        let (endpoint, server) = http::serve(7, move |action, payload| {
            let data = match action {
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                ".future_example" => {
                    assert_eq!(payload["secret"], "synthetic-api-parameter");
                    return serde_json::to_vec(&json!({"status":if rejected {"failed"}else{"ok"},"retcode":if rejected {123}else{0},
                        "data":{"secret":"synthetic-api-result","items":[null,false]},"echo":"opaque","futureField":true})).unwrap();
                }
                _ => panic!("Unexpected synthetic action {action}"),
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
            directory: temp.path().join("unused-installation"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-api".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let _managed = managed::Managed::attach(
            &store,
            &credentials.0,
            "instance-example",
            "owned_api_cli_calls_unknown_actions_and_queries_secret_free_receipts_without_replay",
        );
        let server = server.start();
        let config = temp.path().join("config.json");
        std::fs::write(
            &config,
            serde_json::to_vec(&json!({"stateDir":store.root()})).unwrap(),
        )
        .unwrap();
        let params = temp.path().join("params.json");
        std::fs::write(&params, br#"{"secret":"synthetic-api-parameter"}"#).unwrap();
        let args = [
            "api",
            "call",
            ".future_example",
            "--params-file",
            params.to_str().unwrap(),
            "--request-id",
            "api-example",
        ];
        let (code, value) = run(&config, &args);
        assert_eq!(code, if rejected { 4 } else { 0 }, "{value}");
        assert_eq!(value["data"]["result"]["futureField"], true);
        assert_eq!(value["data"]["result"]["echo"], "opaque");
        assert_eq!(value["meta"]["scope"]["account"], "10001");
        assert_eq!(
            value["data"]["result"]["data"]["items"],
            json!([null, false])
        );
        let schema = qq_cli::cli::registry::get("api.call").unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&value)
        );
        server.join().unwrap();
        let (_, cached) = run(&config, &args);
        assert_eq!(cached["data"]["deduplicated"], true);
        assert_eq!(cached["data"]["resultAvailable"], false);
        let (code, record) = run(&config, &["request", "get", "api-example"]);
        assert_eq!(code, 0);
        assert_eq!(record["data"]["action"], ".future_example");
        assert_eq!(record["data"]["operation"], "api.call");
        assert_eq!(record["data"]["resultAvailable"], false);
        let request_schema = qq_cli::cli::registry::get("request.get").unwrap();
        assert!(
            jsonschema::validator_for(&request_schema["responseSchema"])
                .unwrap()
                .is_valid(&record)
        );
        let text = serde_json::to_string(&record).unwrap();
        assert!(!text.contains("synthetic-api-"));
        std::fs::write(&params, b"{}").unwrap();
        let (code, error) = run(&config, &args);
        assert_eq!(code, 4);
        assert_eq!(error["error"]["code"], "REQUEST_CONFLICT");
    }
}

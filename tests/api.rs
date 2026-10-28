use serde_json::Value;
use std::process::Command;
fn run(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .args(["--config", "missing-api-synthetic.json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn discovery_is_offline_paged_and_reports_unreviewed_boundaries() {
    let first = run(&["api", "list", "--limit", "100"]);
    assert_eq!(first["data"]["total"], 178);
    let token = first["data"]["nextCursor"].as_str().unwrap();
    let next = run(&["api", "list", "--limit", "100", "--cursor", token]);
    assert_eq!(next["data"]["items"].as_array().unwrap().len(), 78);
    let all: std::collections::HashSet<_> = first["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .chain(next["data"]["items"].as_array().unwrap().iter())
        .map(|item| item["action"].as_str().unwrap())
        .collect();
    assert_eq!(all.len(), 178);
    assert!(next["data"].get("nextCursor").is_none());
    let unknown = run(&["api", "describe", "future_example_action"]);
    assert_eq!(unknown["data"]["known"], false);
    assert_eq!(unknown["data"]["requestIdRequired"], true);
    assert_eq!(unknown["data"]["parametersKnown"], false);
    let suffix = run(&["api", "describe", "send_private_msg_rate_limited"]);
    assert_eq!(suffix["data"]["baseAction"], "send_private_msg");
    assert_eq!(suffix["data"]["effect"], "write");
    for args in [
        vec!["api", "list", "--query", "set_", "--cursor", token],
        vec!["api", "describe", "../get_status"],
        vec!["api", "describe", "get_status%2f"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}
#[test]
fn actual_api_discovery_payloads_match_published_schemas() {
    for (path, args) in [
        ("api.list", vec!["api", "list", "--limit", "5"]),
        ("api.describe", vec!["api", "describe", "get_status"]),
    ] {
        let payload = run(&args);
        let schema = qq_cli::cli::registry::get(path).unwrap();
        let validator = jsonschema::validator_for(&schema["responseSchema"]).unwrap();
        assert!(validator.is_valid(&payload), "{payload}");
    }
}

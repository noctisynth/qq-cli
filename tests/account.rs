use serde_json::{Value, json};
use std::process::Command;
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .args(["--config", "missing-synthetic-account.json"])
        .output()
        .unwrap()
}
#[test]
fn account_previews_are_offline_clearable_and_match_real_derived_schemas() {
    for (path, args, fields) in [
        (
            "account.set",
            vec![
                "account",
                "set",
                "--nickname",
                "Synthetic",
                "--signature=",
                "--sex",
                "unknown",
                "--dry-run",
            ],
            json!(["nickname", "signature", "sex"]),
        ),
        (
            "account.set",
            vec!["account", "set", "--signature=", "--dry-run"],
            json!(["signature"]),
        ),
        (
            "account.presence",
            vec!["account", "presence", "--status", "40", "--dry-run"],
            json!(["status", "extStatus", "batteryStatus"]),
        ),
        (
            "account.status-text",
            vec![
                "account",
                "status-text",
                "",
                "--face-id",
                "123",
                "--dry-run",
            ],
            json!(["statusText", "faceId", "faceType"]),
        ),
    ] {
        let output = run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["data"]["requestedFields"], fields);
        assert_eq!(result["data"]["identityVerified"], false);
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&result)
        );
    }
    for args in [
        vec!["account", "set", "--dry-run"],
        vec!["account", "set", "--sex", "male", "--dry-run"],
        vec!["account", "set", "--nickname", "Synthetic"],
    ] {
        assert!(!run(&args).status.success());
    }
    let temp = tempfile::tempdir().unwrap();
    let avatar = temp.path().join("avatar.png");
    std::fs::write(&avatar, b"synthetic-avatar-bytes").unwrap();
    let output = run(&["account", "avatar", avatar.to_str().unwrap(), "--dry-run"]);
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["data"]["inputBytes"], 22);
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("synthetic-avatar-bytes")
    );
}
#[test]
fn reviewed_account_payload_schemas_and_compatibility_descriptions_are_precise() {
    let name = "set_qq_profile".parse().unwrap();
    let info = qq_cli::transport::catalog::describe(&name);
    let value = serde_json::to_value(info).unwrap();
    assert_eq!(value["parametersKnown"], true);
    let validator = jsonschema::validator_for(&value["parameterSchema"]).unwrap();
    assert!(validator.is_valid(&json!({"nickname":"Synthetic","personal_note":"","sex":1})));
    assert!(!validator.is_valid(&json!({"personal_note":""})));
    for action in ["get_online_clients", "_get_model_show", "_set_model_show"] {
        let info = qq_cli::transport::catalog::describe(&action.parse().unwrap());
        assert!(info.compatibility_only);
    }
}

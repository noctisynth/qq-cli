//! Compare public schemas to actual offline parsing and shared semantic rules.
use clap::Parser;
use qq_cli::cli::args::Cli;
use serde_json::{Value, json};
fn check(command: &str, args: Vec<&str>, input: Value, expected: bool) {
    let parsed = Cli::try_parse_from(std::iter::once("qq-cli").chain(args))
        .ok()
        .is_some_and(|cli| cli.validate().is_ok());
    let descriptor = qq_cli::cli::registry::get(command).unwrap();
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&descriptor["inputSchema"])
        .unwrap();
    assert_eq!(parsed, expected, "CLI: {command} {input}");
    assert_eq!(
        validator.is_valid(&input),
        expected,
        "Schema: {command} {input}"
    );
}
#[test]
fn parser_types_and_shared_rules_agree_for_valid_and_invalid_agent_calls() {
    for (args, value, expected) in [
        (
            vec!["contact", "get", "10001"],
            json!({"user_id":"10001"}),
            true,
        ),
        (vec!["contact", "get", "0"], json!({"user_id":"0"}), false),
    ] {
        check("contact.get", args, value, expected);
    }
    check(
        "napcat.start",
        vec!["napcat", "start", "--new"],
        json!({"new":true}),
        false,
    );
    check(
        "napcat.start",
        vec!["napcat", "start", "--new", "--request-id", "example"],
        json!({"new":true,"request_id":"example"}),
        true,
    );
    check(
        "napcat.start",
        vec!["napcat", "start", "--request-id", "example"],
        json!({"request_id":"example"}),
        false,
    );
    check(
        "event.watch",
        vec!["event", "watch", "--resume", "opaque"],
        json!({"resume":"opaque"}),
        false,
    );
    check(
        "group.members",
        vec!["group", "members", "--cursor", "opaque"],
        json!({"cursor":"opaque"}),
        true,
    );
    check(
        "message.send",
        vec![
            "message",
            "send",
            "--conversation",
            "group:20001",
            "--dry-run",
            "--text",
            "hi",
        ],
        json!({"conversation":"group:20001","dry_run":true,"text":"hi"}),
        true,
    );
    check(
        "message.send",
        vec![
            "message",
            "send",
            "--conversation",
            "group:20001",
            "--dry-run",
        ],
        json!({"conversation":"group:20001","dry_run":true}),
        false,
    );
    check(
        "message.send",
        vec![
            "message",
            "send",
            "--conversation",
            "group:20001",
            "--dry-run",
            "--text",
            "hi",
            "--text-file",
            "synthetic.txt",
        ],
        json!({"conversation":"group:20001","dry_run":true,"text":"hi","text_file":"synthetic.txt"}),
        false,
    );
    check(
        "message.search",
        vec![
            "message",
            "search",
            "--conversation",
            "group:20001",
            "--query",
            "",
        ],
        json!({"conversation":"group:20001","query":""}),
        false,
    );
    check(
        "message.history",
        vec![
            "message",
            "history",
            "--conversation",
            "group:20001",
            "--since",
            "not-a-time",
        ],
        json!({"conversation":"group:20001","since":"not-a-time"}),
        false,
    );
    for value in ["0", "1001"] {
        check(
            "contact.list",
            vec!["contact", "list", "--limit", value],
            json!({"limit":value.parse::<u16>().unwrap()}),
            false,
        );
    }
    let mut args = vec![
        "message",
        "send",
        "--conversation",
        "group:20001",
        "--dry-run",
    ];
    for _ in 0..17 {
        args.extend(["--image", "synthetic.png"]);
    }
    check(
        "message.send",
        args,
        json!({"conversation":"group:20001","dry_run":true,"image":vec!["synthetic.png";17]}),
        false,
    );
}

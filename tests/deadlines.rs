use clap::Parser;
use qq_cli::cli::{Cli, Command, MessageCommand};
use qq_cli::client::{Action, Backend};
use qq_cli::config::Config;
use qq_cli::error::Result;
use qq_cli::references::References;
use qq_cli::session::Session;
use qq_cli::state::Instance;
use serde_json::Value;
use std::cell::Cell;
use std::time::{Duration, Instant};

#[test]
fn command_budgets_cover_every_public_timeout_flag() {
    let cases = [
        vec!["contact", "list"],
        vec!["group", "list"],
        vec!["group", "members", "20001"],
        vec!["conversation", "list"],
        vec!["message", "history", "--conversation", "group:20001"],
        vec![
            "message",
            "search",
            "--conversation",
            "group:20001",
            "--query",
            "sample",
        ],
        vec![
            "message",
            "export",
            "--conversation",
            "group:20001",
            "--output",
            "example.ndjson",
        ],
        vec![
            "message",
            "send",
            "--conversation",
            "group:20001",
            "--text",
            "sample",
            "--dry-run",
        ],
        vec![
            "message",
            "reply",
            "synthetic-reference",
            "--text",
            "sample",
            "--dry-run",
        ],
        vec![
            "media",
            "download",
            "synthetic-reference",
            "--output",
            "example.bin",
        ],
        vec!["event", "watch"],
        vec!["napcat", "start"],
    ];
    for args in cases {
        let mut args = [vec!["qq-cli"], args].concat();
        args.extend(["--timeout", "2"]);
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.command.timeout_seconds(), 2);
    }
    assert_eq!(
        Cli::try_parse_from(["qq-cli", "status"])
            .unwrap()
            .command
            .timeout_seconds(),
        30
    );
}

#[test]
fn expired_shared_deadlines_prevent_next_stage_reads_and_output_creation() {
    struct NoCalls(Cell<usize>);
    impl Backend for NoCalls {
        fn call(&self, _: Action, _: &Value, _: Instant) -> Result<Value> {
            self.0.set(self.0.get() + 1);
            panic!("Expired stage called backend")
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("absent-references"), [91; 32]);
    let backend = NoCalls(Cell::new(0));
    let config = Config::default();
    let instance = Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "unused".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    };
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let expired = Instant::now() - Duration::from_millis(1);
    assert_eq!(session.verify(expired).err().unwrap().code, "DEADLINE");
    let cli = Cli::try_parse_from(["qq-cli", "contact", "list"]).unwrap();
    assert_eq!(
        qq_cli::collection::execute_until(
            qq_cli::collection::request(&cli.command).unwrap(),
            &session,
            &refs,
            expired
        )
        .err()
        .unwrap()
        .code,
        "DEADLINE"
    );
    let cli = Cli::try_parse_from(["qq-cli", "contact", "get", "10002"]).unwrap();
    assert_eq!(
        qq_cli::app::execute_read_until(&cli.command, &session, expired)
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    let output = temp.path().join("absent.ndjson");
    let cli = Cli::try_parse_from([
        "qq-cli",
        "message",
        "export",
        "--conversation",
        "group:20001",
        "--output",
        output.to_str().unwrap(),
    ])
    .unwrap();
    let Command::Message {
        command: MessageCommand::Export(args),
    } = cli.command
    else {
        panic!()
    };
    assert_eq!(
        qq_cli::export::execute_until(&args, &session, &refs, expired)
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    assert_eq!(
        qq_cli::media::download("absent-reference", &output, &session, &refs, expired)
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    assert!(!output.exists());
    assert!(!temp.path().join(".qq-cli-export.lock").exists());
    assert!(!temp.path().join("absent-references").exists());
    assert_eq!(backend.0.get(), 0);
    let mut pending = instance.clone();
    pending.state = "prepared".into();
    let pending = Session {
        backend: &backend,
        config: &config,
        instance: &pending,
        expected_version: "4.18.38",
    };
    assert_eq!(
        pending
            .read(
                None,
                Instant::now() + Duration::from_secs(1),
                |_| -> Result<()> { panic!("Unready instance entered a domain operation") }
            )
            .err()
            .unwrap()
            .code,
        "INSTANCE_NOT_READY"
    );
    assert_eq!(backend.0.get(), 0);
}

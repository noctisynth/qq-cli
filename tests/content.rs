use std::time::{Duration, Instant};

use clap::Parser;
use qq_cli::cli::args::{Cli, Command, MessageCommand};
use qq_cli::messaging::content;
use serde_json::json;

#[test]
fn explicit_sources_preserve_literal_text_and_bound_utf8_input() {
    assert_eq!(
        content::text_from_reader(&b"[CQ:at,qq=all]"[..]).unwrap(),
        "[CQ:at,qq=all]"
    );
    for bytes in [vec![], vec![255], vec![b'x'; 65537]] {
        assert!(content::text_from_reader(bytes.as_slice()).is_err());
    }
    assert_eq!(
        content::text_from_reader(vec![b'x'; 65536].as_slice())
            .unwrap()
            .len(),
        65536
    );
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("input.txt");
    std::fs::write(&file, "synthetic file text").unwrap();
    let cli = Cli::try_parse_from([
        "qq-cli",
        "message",
        "send",
        "--conversation",
        "group:20001",
        "--text-file",
        file.to_str().unwrap(),
        "--mention",
        "10002",
        "--dry-run",
    ])
    .unwrap();
    let Command::Message {
        command: MessageCommand::Send(args),
    } = cli.command
    else {
        panic!()
    };
    let segments = content::prepare(
        &args.content,
        &args.conversation,
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    assert_eq!(
        segments,
        vec![
            json!({"type":"text","data":{"text":"synthetic file text"}}),
            json!({"type":"at","data":{"qq":"10002"}})
        ]
    );
}

#[test]
fn text_input_errors_and_private_mentions_fail_during_content_preparation() {
    for args in [
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "group:20001",
            "--image",
            "missing-image.png",
            "--text-file",
            "missing-input.txt",
            "--dry-run",
        ],
        vec![
            "qq-cli",
            "message",
            "send",
            "--conversation",
            "private:10002",
            "--mention",
            "10002",
            "--text-file",
            "missing-input.txt",
            "--dry-run",
        ],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        let Command::Message {
            command: MessageCommand::Send(args),
        } = cli.command
        else {
            panic!()
        };
        let error = content::prepare(
            &args.content,
            &args.conversation,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(matches!(
            error.code,
            "INPUT_INVALID" | "MENTION_UNSUPPORTED"
        ));
    }
}

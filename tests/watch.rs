use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use qq_cli::core::error::{AppError, Result};
use qq_cli::messaging::session::Session;
use qq_cli::messaging::watch::{self, Options, Sink, Source};
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use qq_cli::storage::state::Instance;
use qq_cli::transport::client::{Action, Backend};
use qq_cli::transport::websocket::Frame;
use serde_json::{Value, json};

struct Identity {
    changed: Cell<bool>,
    probes: Cell<usize>,
}
impl Backend for Identity {
    fn call(&self, action: Action, _: &Value, _: Instant) -> Result<Value> {
        self.probes.set(self.probes.get() + 1);
        Ok(match action {
            Action::Status => json!({"online":true}),
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::LoginInfo => json!({"user_id":if self.changed.get(){"10003"}else{"10001"}}),
            _ => panic!("unexpected domain action"),
        })
    }
}
struct Fake<'a> {
    identity: &'a Identity,
    frames: VecDeque<Frame>,
    connects: usize,
    reads: usize,
    disconnects: usize,
    switch: bool,
}
impl Source for Fake<'_> {
    fn connect(&mut self, _: Instant) -> Result<()> {
        self.connects += 1;
        if self.switch && self.connects == 2 {
            self.identity.changed.set(true);
        }
        Ok(())
    }
    fn receive(&mut self) -> Result<Frame> {
        self.reads += 1;
        Ok(self.frames.pop_front().unwrap_or(Frame::Closed))
    }
    fn disconnect(&mut self) {
        self.disconnects += 1;
    }
}
#[derive(Default)]
struct Output {
    bytes: Vec<u8>,
    fail: bool,
}
impl Sink for Output {
    fn record(&mut self, value: &Value, deadline: Instant) -> Result<()> {
        if self.fail && value["type"] == "message" {
            return Err(AppError::new("OUTPUT_IO", "Synthetic output failure", 6));
        }
        watch::write_record(&mut self.bytes, value, deadline)
    }
}
fn instance() -> Instance {
    Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}
fn options(limit: u16) -> Options {
    Options {
        conversation: None,
        limit,
        deadline: Instant::now() + Duration::from_secs(5),
        self_messages: None,
    }
}
fn message(id: &str) -> Frame {
    Frame::Text(serde_json::to_vec(&json!({"post_type":"message","self_id":"10001","user_id":"10002","message_id":id,"time":1700000000,"real_seq":id,"message_type":"group","group_id":"20001","message":[{"type":"text","data":{"text":"synthetic\nevent"}}]})).unwrap())
}
fn lines(output: &Output) -> Vec<Value> {
    std::str::from_utf8(&output.bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn subscription_rechecks_identity_reconnects_with_gap_and_keeps_deduplication() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [75; 32]);
    let identity = Identity {
        changed: Cell::new(false),
        probes: Cell::new(0),
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &identity,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut source = Fake {
        identity: &identity,
        frames: VecDeque::from([
            message("40001"),
            Frame::Closed,
            message("40001"),
            message("40002"),
        ]),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    assert_eq!(
        watch::run(
            &session,
            &refs,
            &mut source,
            &mut output,
            options(2),
            &AtomicBool::new(false)
        )
        .unwrap(),
        0
    );
    let records = lines(&output);
    assert_eq!(
        records
            .iter()
            .map(|line| line["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["ready", "message", "gap", "message", "summary"]
    );
    assert_eq!(records[4]["duplicates"], 1);
    assert_eq!(records[4]["gaps"], 1);
    assert_eq!(source.connects, 2);
    assert_eq!(source.reads, 4);
    assert_eq!(identity.probes.get(), 12); // Four identity observations, three API actions each.
    assert!(source.disconnects >= 2);
}

#[test]
fn reconnect_identity_change_stops_before_reading_new_account() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [76; 32]);
    let identity = Identity {
        changed: Cell::new(false),
        probes: Cell::new(0),
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &identity,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut source = Fake {
        identity: &identity,
        frames: VecDeque::from([Frame::Closed, message("40001")]),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: true,
    };
    let mut output = Output::default();
    assert_eq!(
        watch::run(
            &session,
            &refs,
            &mut source,
            &mut output,
            options(2),
            &AtomicBool::new(false)
        )
        .unwrap(),
        4
    );
    let records = lines(&output);
    assert_eq!(records.last().unwrap()["errorCode"], "ACCOUNT_CHANGED");
    assert_eq!(source.reads, 1);
    assert_eq!(source.connects, 2);
}

#[test]
fn output_failure_cancellation_and_policy_denial_always_disconnect_without_extra_reads() {
    for mode in ["output", "cancel", "policy"] {
        let temp = tempfile::tempdir().unwrap();
        let refs = References::with_key(temp.path().join("refs"), [77; 32]);
        let identity = Identity {
            changed: Cell::new(false),
            probes: Cell::new(0),
        };
        let mut config = Config::default();
        config.permissions.events = mode != "policy";
        let instance = instance();
        let session = Session {
            backend: &identity,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let mut source = Fake {
            identity: &identity,
            frames: VecDeque::from([message("40001"), message("40002")]),
            connects: 0,
            reads: 0,
            disconnects: 0,
            switch: false,
        };
        let mut output = Output {
            fail: mode == "output",
            ..Output::default()
        };
        let error = watch::run(
            &session,
            &refs,
            &mut source,
            &mut output,
            options(2),
            &AtomicBool::new(mode == "cancel"),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            match mode {
                "output" => "OUTPUT_IO",
                "cancel" => "CANCELLED",
                _ => "PERMISSION_DENIED",
            }
        );
        assert_eq!(source.reads, usize::from(mode == "output"));
        assert!(source.disconnects > 0);
        if mode != "output" {
            assert_eq!(identity.probes.get(), 0);
            assert_eq!(source.connects, 0);
        }
    }
}

#[test]
fn repeated_disconnects_exhaust_reconnect_budget_and_report_all_gaps() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [78; 32]);
    let identity = Identity {
        changed: Cell::new(false),
        probes: Cell::new(0),
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &identity,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut source = Fake {
        identity: &identity,
        frames: VecDeque::new(),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    assert_eq!(
        watch::run(
            &session,
            &refs,
            &mut source,
            &mut output,
            options(2),
            &AtomicBool::new(false)
        )
        .unwrap(),
        7
    );
    assert_eq!(source.connects, 3);
    assert_eq!(source.reads, 3);
    let records = lines(&output);
    let summary = records.last().unwrap();
    assert_eq!(summary["reason"], "reconnect_limit");
    assert_eq!(summary["gaps"], 3);
    assert_eq!(
        records
            .iter()
            .filter(|record| record["type"] == "gap")
            .count(),
        3
    );
}

#[test]
fn invalid_limits_and_expired_deadlines_fail_before_identity_or_connection() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [79; 32]);
    let identity = Identity {
        changed: Cell::new(false),
        probes: Cell::new(0),
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &identity,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let mut source = Fake {
        identity: &identity,
        frames: VecDeque::new(),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    for mode in ["limit", "deadline"] {
        let mut options = options(2);
        if mode == "limit" {
            options.limit = 0;
        } else {
            options.deadline = Instant::now();
        }
        let error = watch::run(
            &session,
            &refs,
            &mut source,
            &mut output,
            options,
            &AtomicBool::new(false),
        )
        .unwrap_err();
        assert_eq!(
            error.code,
            if mode == "limit" {
                "INVALID_ARGUMENTS"
            } else {
                "DEADLINE"
            }
        );
    }
    assert_eq!(source.connects, 0);
    assert_eq!(source.reads, 0);
    assert_eq!(identity.probes.get(), 0);
    assert!(output.bytes.is_empty());
}

#[test]
fn signed_resume_backfills_messages_deduplicates_live_and_rejects_scope_changes() {
    struct History {
        identity: Identity,
        calls: Cell<usize>,
        time: i64,
    }
    impl History {
        fn raw(&self, id: &str) -> Value {
            json!({"post_type":"message","self_id":"10001","user_id":"10002","message_id":id,"time":self.time,"real_seq":id,"message_type":"group","group_id":"20001","message":[{"type":"text","data":{"text":"synthetic recovery"}}]})
        }
    }
    impl Backend for History {
        fn call(&self, action: Action, args: &Value, deadline: Instant) -> Result<Value> {
            match action {
                Action::GroupHistory => {
                    self.calls.set(self.calls.get() + 1);
                    assert_eq!(args["group_id"], "20001");
                    Ok(
                        json!({"messages":if args.get("message_seq").is_some(){vec![self.raw("40001")]}else{vec![self.raw("40002"), self.raw("40001")]}}),
                    )
                }
                Action::Message => Ok(self.raw(args["message_id"].as_str().unwrap())),
                _ => self.identity.call(action, args, deadline),
            }
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [76; 32]);
    let config = Config::default();
    let instance = instance();
    let backend = History {
        identity: Identity {
            changed: Cell::new(false),
            probes: Cell::new(0),
        },
        calls: Cell::new(0),
        time: chrono::DateTime::<chrono::Utc>::from(std::time::SystemTime::now()).timestamp(),
    };
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    let mut source = Fake {
        identity: &backend.identity,
        frames: VecDeque::from([Frame::Text(
            serde_json::to_vec(&backend.raw("40001")).unwrap(),
        )]),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    let mut args = options(1);
    args.conversation = Some("group:20001".parse().unwrap());
    watch::run(
        &session,
        &refs,
        &mut source,
        &mut output,
        args,
        &AtomicBool::new(false),
    )
    .unwrap();
    let old = lines(&output);
    let checkpoint = old.last().unwrap()["checkpoint"].as_str().unwrap();
    let mut source = Fake {
        identity: &backend.identity,
        frames: ["40001", "40002", "40003"]
            .map(|id| Frame::Text(serde_json::to_vec(&backend.raw(id)).unwrap()))
            .into(),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    let mut args = options(2);
    args.conversation = Some("group:20001".parse().unwrap());
    watch::run_resuming(
        &session,
        &refs,
        &mut source,
        &mut output,
        args,
        &AtomicBool::new(false),
        Some(checkpoint),
    )
    .unwrap();
    let records = lines(&output);
    let messages = records
        .iter()
        .filter(|v| v["type"] == "message")
        .collect::<Vec<_>>();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["source"], "history");
    assert_eq!(messages[1]["source"], "live");
    assert_eq!(records[1]["type"], "gap");
    assert_eq!(records.last().unwrap()["duplicates"], 3);
    assert_eq!(records.last().unwrap()["recovery"]["emitted"], 1);
    let schema = qq_cli::cli::registry::get("event.watch").unwrap();
    let validator = jsonschema::validator_for(&schema["responseSchema"]).unwrap();
    for record in records {
        assert!(validator.is_valid(&record), "{record}");
    }
    let before = backend.calls.get();
    let mut args = options(2);
    args.conversation = Some("group:20002".parse().unwrap());
    source.connects = 0;
    assert_eq!(
        watch::run_resuming(
            &session,
            &refs,
            &mut source,
            &mut Output::default(),
            args,
            &AtomicBool::new(false),
            Some(checkpoint)
        )
        .unwrap_err()
        .code,
        "EVENT_CHECKPOINT_STALE"
    );
    assert_eq!(source.connects, 0);
    assert_eq!(backend.calls.get(), before);

    // A normal reconnect also backfills a selected conversation without --resume.
    let mut args = options(2);
    args.conversation = Some("group:20001".parse().unwrap());
    let mut source = Fake {
        identity: &backend.identity,
        frames: VecDeque::from([
            Frame::Text(serde_json::to_vec(&backend.raw("40001")).unwrap()),
            Frame::Closed,
        ]),
        connects: 0,
        reads: 0,
        disconnects: 0,
        switch: false,
    };
    let mut output = Output::default();
    watch::run(
        &session,
        &refs,
        &mut source,
        &mut output,
        args,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(source.connects, 2);
    assert_eq!(lines(&output).last().unwrap()["recovery"]["emitted"], 1);
}

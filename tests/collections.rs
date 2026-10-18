use std::cell::{Cell, RefCell};
use std::time::Instant;

use clap::Parser;
use qq_cli::cli::args::Cli;
use qq_cli::core::error::Result;
use qq_cli::messaging::collection;
use qq_cli::messaging::session::Session;
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use qq_cli::storage::state::Instance;
use qq_cli::transport::client::{Action, Backend};
use serde_json::{Value, json};

struct Fake {
    calls: RefCell<Vec<String>>,
    identity_reads: Cell<usize>,
    switch_after: Option<usize>,
    wrong_group: bool,
}

impl Fake {
    fn new() -> Self {
        Self {
            calls: RefCell::new(vec![]),
            identity_reads: Cell::new(0),
            switch_after: None,
            wrong_group: false,
        }
    }
}

impl Backend for Fake {
    fn call(&self, action: Action, _: &Value, _: Instant) -> Result<Value> {
        self.calls.borrow_mut().push(action.name().into());
        Ok(match action {
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::LoginInfo => {
                self.identity_reads.set(self.identity_reads.get() + 1);
                let switched = self
                    .switch_after
                    .is_some_and(|after| self.identity_reads.get() > after);
                json!({"user_id":if switched {"10002"} else {"10001"},"nickname":"Synthetic"})
            }
            Action::Status => json!({"online":true,"good":true}),
            Action::FriendList => json!([
                {"user_id":"10002","nickname":"Sample Two"},
                {"user_id":"10001","nickname":"Sample One"},
                {"user_id":"10003","nickname":"Other"}
            ]),
            Action::GroupList => json!([{ "group_id":"20001","group_name":"Example Group" }]),
            Action::GroupInfo => {
                json!({"group_id":if self.wrong_group {"20002"}else{"20001"},"group_name":"Example Group"})
            }
            Action::GroupMembers => {
                json!([{"user_id":"10002","role":"member","nickname":"Sample"}])
            }
            Action::RecentContacts => {
                json!([{ "peerUin":"20001","chatType":2,"msgTime":"1700000000","peerName":"Recent Example",
                "lastestMsg":{"message":"synthetic-private-body"} }])
            }
            _ => panic!("unexpected action in read contract test"),
        })
    }
}

#[test]
fn public_read_schemas_validate_actual_gets_collections_and_pagination_constraints() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [84; 32]);
    let backend = Fake::new();
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    for (name, args) in [
        ("contact.list", vec!["contact", "list", "--limit", "1"]),
        ("contact.get", vec!["contact", "get", "10002"]),
        ("group.list", vec!["group", "list"]),
        ("group.get", vec!["group", "get", "20001"]),
        ("group.members", vec!["group", "members", "20001"]),
        ("conversation.list", vec!["conversation", "list"]),
        (
            "conversation.get",
            vec!["conversation", "get", "group:20001"],
        ),
    ] {
        let cli = Cli::try_parse_from(std::iter::once("qq-cli").chain(args)).unwrap();
        cli.validate().unwrap();
        let output = if let Some(request) = collection::request(&cli.command) {
            collection::execute(request, &session, &refs).unwrap()
        } else {
            qq_cli::app::execute_read(&cli.command, &session).unwrap()
        };
        let value = serde_json::to_value(output).unwrap();
        let description = qq_cli::cli::registry::get(name).unwrap();
        let validator = jsonschema::validator_for(&description["responseSchema"]).unwrap();
        assert!(validator.is_valid(&value), "{name}: {value}");
        let mut invalid = value.clone();
        invalid["meta"]["scope"]["account"] = json!(10001);
        assert!(!validator.is_valid(&invalid));
        let mut invalid = value.clone();
        if invalid["data"].is_array() {
            invalid["data"][0]["private_url"] = json!("hidden");
        } else {
            invalid["data"]["private_url"] = json!("hidden");
        }
        assert!(!validator.is_valid(&invalid));
        if name == "contact.list" {
            let mut invalid = value.clone();
            invalid["pageInfo"]["nextCursor"] = json!(null);
            assert!(!validator.is_valid(&invalid));
        }
        let error = serde_json::to_value(qq_cli::core::output::Envelope::failure(
            qq_cli::core::error::AppError::invalid(),
        ))
        .unwrap();
        assert!(validator.is_valid(&error));
    }
}

#[test]
fn group_get_rejects_wrong_target_and_recent_activity_rejects_millisecond_timestamps() {
    let mut backend = Fake::new();
    backend.wrong_group = true;
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let cli = Cli::try_parse_from(["qq-cli", "group", "get", "20001"]).unwrap();
    assert_eq!(
        qq_cli::app::execute_read(&cli.command, &session)
            .err()
            .unwrap()
            .code,
        "BACKEND_PROTOCOL"
    );
    assert!(
        qq_cli::messaging::read::recent(
            &json!({"peerUin":"20001","chatType":2,"msgTime":"1700000000000"})
        )
        .is_err()
    );
}

fn instance() -> Instance {
    Instance {
        id: "instance-example".into(),
        installation_id: "install-example".into(),
        endpoint: "http://127.0.0.1:16000/".into(),
        ws_endpoint: None,
        credential_key: "synthetic-reference".into(),
        account: Some("10001".into()),
        run_id: "run-example".into(),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}

#[test]
fn a_normal_full_page_succeeds_and_cursor_restores_filter_without_body_storage() {
    let temp = tempfile::tempdir().unwrap();
    let references = References::with_key(temp.path().join("references"), [42; 32]);
    let backend = Fake::new();
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "v4.18.38",
    };
    let command = Cli::try_parse_from([
        "qq-cli", "contact", "list", "--query", "Sample", "--limit", "1",
    ])
    .unwrap();
    let page = collection::execute(
        collection::request(&command.command).unwrap(),
        &session,
        &references,
    )
    .unwrap();
    assert_eq!(page.exit_code(), 0);
    let page = serde_json::to_value(page).unwrap();
    assert_eq!(page["data"][0]["userId"], "10001");
    assert_eq!(page["pageInfo"]["complete"], false);
    let cursor = page["pageInfo"]["nextCursor"].as_str().unwrap();
    assert!(cursor.len() < 80, "opaque handles should be compact");
    let command = Cli::try_parse_from([
        "qq-cli", "contact", "list", "--cursor", cursor, "--limit", "1",
    ])
    .unwrap();
    assert_eq!(
        collection::inferred_instance(&command.command, &references)
            .unwrap()
            .as_deref(),
        Some("instance-example")
    );
    let next = collection::execute(
        collection::request(&command.command).unwrap(),
        &session,
        &references,
    )
    .unwrap();
    let next = serde_json::to_value(next).unwrap();
    assert_eq!(next["data"][0]["userId"], "10002");
    assert_eq!(next["pageInfo"]["complete"], true);
    for file in std::fs::read_dir(temp.path().join("references")).unwrap() {
        let metadata = std::fs::read_to_string(file.unwrap().path()).unwrap();
        assert!(!metadata.contains("Sample One"));
        assert!(!metadata.contains("Sample Two"));
    }
}

#[test]
fn recent_filter_does_not_expand_into_contacts_or_groups() {
    let temp = tempfile::tempdir().unwrap();
    let references = References::with_key(temp.path().join("references"), [42; 32]);
    let backend = Fake::new();
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let command =
        Cli::try_parse_from(["qq-cli", "conversation", "list", "--query", "Sample"]).unwrap();
    let result = collection::execute(
        collection::request(&command.command).unwrap(),
        &session,
        &references,
    )
    .unwrap();
    let result = serde_json::to_value(result).unwrap();
    assert!(result["data"].as_array().unwrap().is_empty());
    assert_eq!(result["pageInfo"]["coverage"], "recent");
    assert!(
        !backend
            .calls
            .borrow()
            .iter()
            .any(|name| matches!(name.as_str(), "get_friend_list" | "get_group_list"))
    );
    assert!(!result.to_string().contains("synthetic-private-body"));
}

#[test]
fn policy_wrong_account_and_mid_read_account_changes_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let references = References::with_key(temp.path().join("references"), [42; 32]);
    let command = Cli::try_parse_from(["qq-cli", "contact", "list"]).unwrap();
    let mut config = Config::default();
    config.permissions.read = false;
    let instance = instance();
    let backend = Fake::new();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert_eq!(
        collection::execute(
            collection::request(&command.command).unwrap(),
            &session,
            &references
        )
        .err()
        .unwrap()
        .code,
        "PERMISSION_DENIED"
    );
    assert!(backend.calls.borrow().is_empty());
    config.permissions.read = true;
    for switch_after in [0, 1] {
        let mut backend = Fake::new();
        backend.switch_after = Some(switch_after);
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let error = collection::execute(
            collection::request(&command.command).unwrap(),
            &session,
            &references,
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "ACCOUNT_CHANGED");
        let reads = backend
            .calls
            .borrow()
            .iter()
            .filter(|action| action.as_str() == "get_friend_list")
            .count();
        assert_eq!(reads, switch_after);
    }
}

#[test]
fn references_detect_metadata_tampering_and_cross_state_handles() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("references");
    let references = References::with_key(directory.clone(), [42; 32]);
    let token = references
        .issue("m1", &json!({"account":"10001","id":"-123"}))
        .unwrap();
    assert_eq!(references.get::<Value>("m1", &token).unwrap()["id"], "-123");
    let foreign = References::with_key(directory.clone(), [43; 32]);
    assert!(foreign.get::<Value>("m1", &token).is_err());
    let path = std::fs::read_dir(directory)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(path, b"{\"account\":\"10002\",\"id\":\"-123\"}").unwrap();
    assert!(references.get::<Value>("m1", &token).is_err());
    assert!(references.get::<Value>("m1", "m1.../escape").is_err());
}

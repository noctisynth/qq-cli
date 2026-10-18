use qq_cli::core::domain::Scope;
use qq_cli::messaging::attachments::{self, Kind, Retained};
use qq_cli::requests::ledger::{self, Begin, Outcome};
use qq_cli::storage::references::References;
use qq_cli::storage::state::Store;
use serde_json::json;
use std::time::{Duration, Instant, SystemTime};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn scope() -> Scope {
    Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    }
}
fn prepare(
    temp: &std::path::Path,
    store: &Store,
    refs: &References,
    key: &str,
    outcome: Outcome,
) -> Retained {
    let source = temp.join(format!("{key}.bin"));
    std::fs::write(&source, b"synthetic cached bytes").unwrap();
    let retained = attachments::prepare(&source, Kind::File, store, deadline())
        .unwrap()
        .retain(store, refs, key, &scope(), deadline())
        .unwrap();
    let Begin::Fresh(intent) = ledger::begin(
        store,
        key,
        "send",
        &scope(),
        &json!({"fixture":key}),
        deadline(),
    )
    .unwrap() else {
        panic!("duplicate synthetic request")
    };
    if outcome == Outcome::Pending {
        drop(intent);
    } else {
        intent.finish(outcome, None).unwrap();
    }
    retained
}
fn old() -> SystemTime {
    SystemTime::now() + Duration::from_secs(2 * 24 * 60 * 60)
}

#[test]
fn cache_prune_keeps_uncertain_active_and_young_snapshots_and_is_scope_filtered() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [88; 32]);
    let accepted = prepare(temp.path(), &store, &refs, "accepted", Outcome::Accepted);
    let rejected = prepare(temp.path(), &store, &refs, "rejected", Outcome::Rejected);
    let pending = prepare(temp.path(), &store, &refs, "pending", Outcome::Pending);
    let unknown = prepare(temp.path(), &store, &refs, "unknown", Outcome::Unknown);
    let young = qq_cli::messaging::attachments::cache::prune(
        &store,
        &refs,
        None,
        SystemTime::now(),
        deadline(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(young).unwrap()["data"]["removedFiles"],
        0
    );
    let other = qq_cli::messaging::attachments::cache::prune(
        &store,
        &refs,
        Some("other-instance"),
        old(),
        deadline(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(other).unwrap()["data"]["removedFiles"],
        0
    );
    let result =
        qq_cli::messaging::attachments::cache::prune(&store, &refs, None, old(), deadline())
            .unwrap();
    let value = serde_json::to_value(result).unwrap();
    let schema = qq_cli::cli::registry::get("media.prune").unwrap();
    assert!(
        jsonschema::validator_for(&schema["responseSchema"])
            .unwrap()
            .is_valid(&value)
    );
    assert_eq!(value["data"]["removedFiles"], 2);
    assert_eq!(value["data"]["preservedEntries"], 2);
    assert!(!accepted.directory().exists());
    assert!(!rejected.directory().exists());
    assert!(pending.path().exists());
    assert!(unknown.path().exists());
    assert!(ledger::get(&store, "accepted").is_ok());
}

#[test]
fn removal_journal_recovers_partial_cleanup_and_never_deletes_unknown_files() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(temp.path().join("refs"), [89; 32]);
    let interrupted = prepare(temp.path(), &store, &refs, "interrupted", Outcome::Accepted);
    let foreign = prepare(temp.path(), &store, &refs, "foreign", Outcome::Accepted);
    std::fs::write(foreign.directory().join("unowned.bin"), b"must remain").unwrap();
    let journals = store.root().join("attachment-removals");
    qq_cli::storage::state::private_dir(&journals).unwrap();
    let journal = journals.join(format!(
        "{}.json",
        interrupted
            .directory()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
    ));
    std::fs::copy(
        interrupted.directory().join(".qq-cli-attachment.json"),
        &journal,
    )
    .unwrap();
    std::fs::remove_file(interrupted.path()).unwrap();
    std::fs::remove_file(interrupted.directory().join(".qq-cli-attachment.json")).unwrap();
    let result =
        qq_cli::messaging::attachments::cache::prune(&store, &refs, None, old(), deadline())
            .unwrap();
    let value = serde_json::to_value(result).unwrap();
    assert_eq!(value["data"]["removedEntries"], 1);
    assert_eq!(value["data"]["preservedEntries"], 1);
    assert!(!journal.exists());
    assert!(!interrupted.directory().exists());
    assert!(foreign.path().exists());
    assert!(foreign.directory().join("unowned.bin").exists());
}

#[test]
fn media_prune_without_any_cache_is_offline_and_does_not_initialize_state() {
    use clap::Parser;
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config.json");
    std::fs::write(&config, r#"{"stateDir":"state"}"#).unwrap();
    let cli = qq_cli::cli::args::Cli::try_parse_from([
        "qq-cli",
        "media",
        "prune",
        "--config",
        config.to_str().unwrap(),
    ])
    .unwrap();
    let output = qq_cli::app::execute(cli);
    assert_eq!(output.exit_code(), 0);
    assert_eq!(
        serde_json::to_value(output).unwrap()["data"]["removedFiles"],
        0
    );
    assert!(!temp.path().join("state").exists());
}

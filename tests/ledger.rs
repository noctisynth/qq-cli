use std::time::{Duration, Instant};

use qq_cli::domain::Scope;
use qq_cli::ledger::{self, Begin, Outcome};
use qq_cli::state::Store;
use serde_json::json;

fn scope(instance: &str, account: &str) -> Scope {
    Scope {
        instance_id: instance.into(),
        account: account.into(),
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn accepted_keys_are_global_per_state_and_do_not_store_message_bodies() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    assert_eq!(
        ledger::get(&store, "synthetic-key").unwrap_err().code,
        "REQUEST_NOT_FOUND"
    );
    assert!(!store.root().exists());
    let payload = json!({"conversation":"group:20001","text":"synthetic body"});
    let Begin::Fresh(intent) = ledger::begin(
        &store,
        "synthetic-key",
        "send",
        &scope("one", "10001"),
        &payload,
        deadline(),
    )
    .unwrap() else {
        panic!("fresh key expected")
    };
    assert_eq!(
        ledger::get(&store, "synthetic-key").unwrap().outcome,
        Outcome::Pending
    );
    intent
        .finish(Outcome::Accepted, Some(json!({"messageId":"50001"})))
        .unwrap();
    let Begin::Existing(record) = ledger::begin(
        &store,
        "synthetic-key",
        "send",
        &scope("two", "10001"),
        &payload,
        deadline(),
    )
    .unwrap() else {
        panic!("must not replay")
    };
    assert_eq!(record.outcome, Outcome::Accepted);
    assert_eq!(record.scope.instance_id, "one");
    for file in std::fs::read_dir(store.root().join("requests")).unwrap() {
        assert!(
            !std::fs::read_to_string(file.unwrap().path())
                .unwrap()
                .contains("synthetic body")
        );
    }
    for (operation, account, payload) in [
        ("reply", "10001", payload.clone()),
        ("send", "10002", payload.clone()),
        ("send", "10001", json!({"text":"changed"})),
    ] {
        assert_eq!(
            ledger::begin(
                &store,
                "synthetic-key",
                operation,
                &scope("one", account),
                &payload,
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "REQUEST_CONFLICT"
        );
    }
}

#[test]
fn crash_residue_becomes_unknown_and_cannot_be_claimed_again() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let scope = scope("one", "10001");
    let payload = json!({"text":"synthetic crash body"});
    let Begin::Fresh(intent) =
        ledger::begin(&store, "crash-key", "send", &scope, &payload, deadline()).unwrap()
    else {
        panic!()
    };
    drop(intent); // Models termination after durable pending, before acknowledgement persistence.
    for _ in 0..2 {
        let Begin::Existing(record) =
            ledger::begin(&store, "crash-key", "send", &scope, &payload, deadline()).unwrap()
        else {
            panic!("must not replay")
        };
        assert_eq!(record.outcome, Outcome::Unknown);
    }
    assert_eq!(
        ledger::get(&store, "crash-key").unwrap().outcome,
        Outcome::Unknown
    );
}

#[test]
fn lock_deadline_protects_concurrent_writers_and_failed_receipt_stays_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let scope = scope("one", "10001");
    let payload = json!({"text":"sample"});
    let Begin::Fresh(intent) =
        ledger::begin(&store, "locked-key", "send", &scope, &payload, deadline()).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        ledger::begin(
            &store,
            "locked-key",
            "send",
            &scope,
            &payload,
            Instant::now() + Duration::from_millis(30)
        )
        .err()
        .unwrap()
        .code,
        "STATE_BUSY"
    );
    // Replace the record directory after intent persistence to inject final-commit failure.
    std::fs::rename(
        store.root().join("requests"),
        store.root().join("saved-requests"),
    )
    .unwrap();
    std::fs::write(store.root().join("requests"), b"synthetic obstruction").unwrap();
    let error = intent
        .finish(Outcome::Accepted, Some(json!({"messageId":"50002"})))
        .unwrap_err();
    assert_eq!(error.code, "WRITE_OUTCOME_UNKNOWN");
    assert_eq!(error.outcome, Some("unknown"));
    assert!(!error.retry_safe);
    std::fs::remove_file(store.root().join("requests")).unwrap();
    std::fs::rename(
        store.root().join("saved-requests"),
        store.root().join("requests"),
    )
    .unwrap();
    let Begin::Existing(record) =
        ledger::begin(&store, "locked-key", "send", &scope, &payload, deadline()).unwrap()
    else {
        panic!()
    };
    assert_eq!(record.outcome, Outcome::Unknown);
}

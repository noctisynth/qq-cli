use std::time::{Duration, Instant};

use qq_cli::config::Config;
use qq_cli::domain::{Conversation, Scope};
use qq_cli::events::Watch;
use qq_cli::references::References;
use serde_json::{Value, json};

fn scope() -> Scope {
    Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
fn event() -> Value {
    json!({"post_type":"message","self_id":"10001","time":1700000000,"message_id":"40001","real_seq":"101","message_type":"group","group_id":"20001","user_id":"10002","message":[{"type":"text","data":{"text":"synthetic event"}}],"raw_message":"private raw field"})
}
fn ingest(watch: &mut Watch<'_>, value: &Value) -> qq_cli::error::Result<Option<Value>> {
    let result = watch.ingest(&serde_json::to_vec(value).unwrap());
    if let Ok(Some(record)) = &result {
        let schema = qq_cli::schema::event_stream();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(
            validator.is_valid(record),
            "Event violates published schema: {record}"
        );
    }
    result
}

#[test]
fn event_stream_scopes_normalizes_deduplicates_and_reports_gaps() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [71; 32]);
    let config = Config::default();
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        10,
        deadline(),
    )
    .unwrap();
    assert_eq!(watch.ready(None)["selfMessages"], Value::Null);
    assert_eq!(watch.ready(Some(false))["scope"]["account"], "10001");
    let first = ingest(&mut watch, &event()).unwrap().unwrap();
    assert_eq!(first["direction"], "incoming");
    assert!(
        !serde_json::to_string(&first)
            .unwrap()
            .contains("private raw field")
    );
    let mut numeric = event();
    numeric["message_id"] = json!(40001);
    numeric["real_seq"] = json!(101);
    numeric["user_id"] = json!(10002);
    assert!(ingest(&mut watch, &numeric).unwrap().is_none());
    assert!(
        ingest(
            &mut watch,
            &json!({"post_type":"meta_event","meta_event_type":"heartbeat","self_id":"10001"})
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(watch.gap("reconnected")["recovered"], false);
    let summary = watch.summary("deadline");
    assert_eq!(summary["emitted"], 1);
    assert_eq!(summary["duplicates"], 1);
    assert_eq!(summary["heartbeats"], 1);
    assert_eq!(summary["gaps"], 1);
    assert_eq!(summary["coverage"], "best_effort");
}

#[test]
fn event_stream_filters_before_reference_creation_and_stops_on_identity_change() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [72; 32]);
    let mut config = Config::default();
    config.permissions.targets = vec!["group:20002".into()];
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        10,
        deadline(),
    )
    .unwrap();
    assert!(ingest(&mut watch, &event()).unwrap().is_none());
    assert!(!temp.path().join("refs").exists());
    let mut other = event();
    other["self_id"] = json!("10003");
    assert_eq!(
        ingest(&mut watch, &other).unwrap_err().code,
        "ACCOUNT_CHANGED"
    );
    assert_eq!(
        ingest(&mut watch, &event()).unwrap_err().code,
        "EVENT_STREAM_STOPPED"
    );
    assert_eq!(
        Watch::new(
            scope(),
            "run-example".into(),
            Some(Conversation::Group("20001".into())),
            &config,
            &refs,
            10,
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "TARGET_DENIED"
    );
}

#[test]
fn outgoing_private_events_use_recipient_and_stream_limits_are_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [73; 32]);
    let config = Config::default();
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        Some(Conversation::Private("10002".into())),
        &config,
        &refs,
        1,
        deadline(),
    )
    .unwrap();
    let mut sent = event();
    sent["post_type"] = json!("message_sent");
    sent["message_type"] = json!("private");
    sent["user_id"] = json!("10001");
    sent["target_id"] = json!("10002");
    let result = ingest(&mut watch, &sent).unwrap().unwrap();
    assert_eq!(result["direction"], "outgoing");
    assert_eq!(result["message"]["conversation"], "private:10002");
    assert!(watch.full());
    assert_eq!(
        ingest(&mut watch, &event()).unwrap_err().code,
        "EVENT_STREAM_LIMIT"
    );
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        1,
        Instant::now(),
    )
    .unwrap();
    assert_eq!(ingest(&mut watch, &event()).unwrap_err().code, "DEADLINE");
    let mut small = config.clone();
    small.response_bytes = 1;
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &small,
        &refs,
        1,
        deadline(),
    )
    .unwrap();
    assert_eq!(
        ingest(&mut watch, &event()).unwrap_err().code,
        "EVENT_STREAM_LIMIT"
    );
}

#[test]
fn unsupported_notices_do_not_leak_raw_payloads_and_frame_budgets_are_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [74; 32]);
    let config = Config::default();
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        10,
        deadline(),
    )
    .unwrap();
    let notice = json!({"self_id":"10001","post_type":"notice","notice_type":"unrecognized_notice","group_id":"20001","user_id":"10002","time":1700000000,"message_id":"40001","private_url":"https://invalid.example/?token=synthetic-secret"});
    let result = ingest(&mut watch, &notice).unwrap().unwrap();
    assert_eq!(result["type"], "notice");
    assert_eq!(result["supported"], false);
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("synthetic-secret")
    );
    assert!(!temp.path().join("refs").exists());
    let heartbeat =
        json!({"self_id":"10001","post_type":"meta_event","meta_event_type":"heartbeat"});
    for _ in 1..10_000 {
        assert!(ingest(&mut watch, &heartbeat).unwrap().is_none());
    }
    assert_eq!(
        ingest(&mut watch, &heartbeat).unwrap_err().code,
        "EVENT_STREAM_LIMIT"
    );
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        1,
        deadline(),
    )
    .unwrap();
    assert_eq!(
        watch
            .ingest(&vec![b' '; 4 * 1024 * 1024 + 1])
            .unwrap_err()
            .code,
        "EVENT_STREAM_LIMIT"
    );
    let mut denied = config.clone();
    denied.permissions.events = false;
    assert_eq!(
        Watch::new(
            scope(),
            "run-example".into(),
            None,
            &denied,
            &refs,
            1,
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "PERMISSION_DENIED"
    );
}

#[test]
fn known_notices_are_typed_and_private_notices_respect_conversation_filter() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [81; 32]);
    let config = Config::default();
    for kind in [
        "group_recall",
        "friend_recall",
        "group_increase",
        "group_decrease",
        "group_admin",
        "friend_add",
    ] {
        let private = kind.starts_with("friend");
        let target = if private {
            Conversation::Private("10002".into())
        } else {
            Conversation::Group("20001".into())
        };
        let mut watch = Watch::new(
            scope(),
            "run-example".into(),
            Some(target.clone()),
            &config,
            &refs,
            10,
            deadline(),
        )
        .unwrap();
        let mut notice = json!({"post_type":"notice","self_id":"10001","notice_type":kind,"time":1700000000,"user_id":"10002","message_id":"-40001","operator_id":"10003","private_url":"https://invalid.example/?token=synthetic-secret","sub_type":match kind {"group_increase"=>"approve","group_decrease"=>"kick",_=>"set"}});
        if !private {
            notice["group_id"] = json!("20001");
        }
        let result = ingest(&mut watch, &notice).unwrap().unwrap();
        assert_eq!(result["supported"], true, "{kind}");
        assert_eq!(result["notice"]["kind"], kind);
        assert_eq!(result["conversation"], json!(target));
        assert!(
            !serde_json::to_string(&result)
                .unwrap()
                .contains("synthetic-secret")
        );
        if private {
            notice["user_id"] = json!("10004");
            assert!(ingest(&mut watch, &notice).unwrap().is_none());
        }
    }
    assert!(!temp.path().join("refs").exists());
}

#[test]
fn malformed_known_notice_fields_stop_stream_and_unknown_subtypes_remain_unsupported() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [82; 32]);
    let config = Config::default();
    for (field, value) in [
        ("operator_id", json!(null)),
        ("time", json!(1700000000000_u64)),
        ("user_id", json!("-2")),
    ] {
        let mut notice = json!({"post_type":"notice","self_id":"10001","notice_type":"group_recall","time":1700000000,"group_id":"20001","user_id":"10002","operator_id":"10003","message_id":"40001"});
        notice[field] = value;
        let mut watch = Watch::new(
            scope(),
            "run-example".into(),
            None,
            &config,
            &refs,
            10,
            deadline(),
        )
        .unwrap();
        assert_eq!(
            ingest(&mut watch, &notice).unwrap_err().code,
            "BACKEND_PROTOCOL"
        );
    }
    let notice = json!({"post_type":"notice","self_id":"10001","notice_type":"group_admin","time":1700000000,"group_id":"20001","user_id":"10002","sub_type":"future-subtype"});
    let mut watch = Watch::new(
        scope(),
        "run-example".into(),
        None,
        &config,
        &refs,
        10,
        deadline(),
    )
    .unwrap();
    assert_eq!(
        ingest(&mut watch, &notice).unwrap().unwrap()["supported"],
        false
    );
    let unresolved = json!({"post_type":"notice","self_id":"10001","notice_type":"group_increase","time":1700000000,"group_id":"20001","user_id":0,"operator_id":0,"sub_type":"approve"});
    let event = ingest(&mut watch, &unresolved).unwrap().unwrap();
    assert!(event["notice"]["userId"].is_null());
    assert!(event["notice"]["operatorId"].is_null());
}

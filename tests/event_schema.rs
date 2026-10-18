use qq_cli::core::domain::Scope;
use qq_cli::messaging::events::Watch;
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn published_event_schema_validates_real_records_and_rejects_contract_violations() {
    let description = qq_cli::cli::registry::get("event.watch").unwrap();
    assert_eq!(description["outputFormat"], "ndjson");
    let validator = jsonschema::validator_for(&description["responseSchema"]).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [83; 32]);
    let config = Config::default();
    let mut watch = Watch::new(
        Scope {
            instance_id: "instance-example".into(),
            account: "10001".into(),
        },
        "run-example".into(),
        None,
        &config,
        &refs,
        10,
        Instant::now() + Duration::from_secs(5),
    )
    .unwrap();
    assert!(validator.is_valid(&watch.ready(None)));
    assert!(validator.is_valid(&watch.gap("connection_interrupted")));
    assert!(validator.is_valid(&watch.summary("limit")));
    let raw = json!({"post_type":"message","message_type":"group","self_id":"10001","group_id":"20001","user_id":"10002","time":1700000000,"message_id":"40001","message":[
        {"type":"text","data":{"text":"synthetic schema message"}},
        {"type":"at","data":{"qq":"10003"}},
        {"type":"at","data":{"qq":"all"}},
        {"type":"reply","data":{"id":"40000"}},
        {"type":"image","data":{"file":"fixture.jpg","file_size":"12"}},
        {"type":"file","data":{"file_id":"file-id","file":"fixture.bin"}},
        {"type":"forward","data":{"id":"forward-resource"}},
        {"type":"future-kind","data":{"private":"never exposed"}}
    ]});
    let event = watch
        .ingest(&serde_json::to_vec(&raw).unwrap())
        .unwrap()
        .unwrap();
    assert!(validator.is_valid(&event), "{event}");
    for path in ["id", "messageRef", "conversation"] {
        let mut invalid = event.clone();
        invalid["message"][path] = json!(40001);
        assert!(!validator.is_valid(&invalid), "Numeric {path} accepted");
    }
    let mut invalid = event.clone();
    invalid["private_url"] = json!("https://invalid.example/");
    assert!(!validator.is_valid(&invalid));
    let mut invalid = event.clone();
    invalid["message"]["segments"][5]
        .as_object_mut()
        .unwrap()
        .remove("mediaRef");
    assert!(!validator.is_valid(&invalid));
    let mut invalid = watch.ready(None);
    invalid["scope"]["account"] = json!(10001);
    assert!(!validator.is_valid(&invalid));
    let envelope = serde_json::to_value(qq_cli::core::output::Envelope::failure(
        qq_cli::core::error::AppError::invalid(),
    ))
    .unwrap();
    assert!(
        jsonschema::validator_for(&description["preStreamErrorSchema"])
            .unwrap()
            .is_valid(&envelope)
    );
    assert!(!validator.is_valid(&envelope));
}

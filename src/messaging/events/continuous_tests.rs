use super::*;
use std::time::Duration;

fn notice(time: u64) -> Vec<u8> {
    serde_json::to_vec(&json!({"post_type":"notice","notice_type":"friend_add","self_id":"10001","user_id":"10002","time":time})).unwrap()
}

#[test]
fn continuous_events_keep_deduplication_bounded_and_cross_total_frame_and_byte_caps() {
    let tmp = tempfile::tempdir().unwrap();
    let refs = References::with_key(tmp.path().join("refs"), [71; 32]);
    let config = Config {
        response_bytes: 1024,
        ..Config::default()
    };
    let mut watch = Watch::new(
        Scope {
            instance_id: "instance-example".into(),
            account: "10001".into(),
        },
        "run-example".into(),
        None,
        &config,
        &refs,
        1,
        Instant::now() + Duration::from_secs(60),
    )
    .unwrap();
    watch.continuous();
    for n in 0..5000 {
        assert!(watch.ingest(&notice(1700000000 + n)).unwrap().is_some());
    }
    assert!(!watch.full());
    assert_eq!(watch.seen.len(), 4096);
    assert_eq!(watch.seen_order.len(), 4096);
    assert!(watch.bytes > config.response_bytes);
    assert!(watch.ingest(&notice(1700004999)).unwrap().is_none());
    assert!(watch.ingest(&notice(1700000000)).unwrap().is_some());
    let heartbeat =
        br#"{"self_id":"10001","post_type":"meta_event","meta_event_type":"heartbeat"}"#;
    for _ in 0..10001 {
        assert!(watch.ingest(heartbeat).unwrap().is_none());
    }
    assert!(watch.frames > 10000);
    assert_eq!(watch.seen.len(), 4096);
    assert_eq!(watch.emitted, 5001);
}

#[test]
fn continuous_events_still_enforce_single_record_and_frame_budgets() {
    let tmp = tempfile::tempdir().unwrap();
    let refs = References::with_key(tmp.path().join("refs"), [71; 32]);
    let config = Config {
        response_bytes: 64,
        ..Config::default()
    };
    for oversized in [false, true] {
        let mut watch = Watch::new(
            Scope {
                instance_id: "instance-example".into(),
                account: "10001".into(),
            },
            "run-example".into(),
            None,
            &config,
            &refs,
            1,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        watch.continuous();
        let frame = if oversized {
            vec![b' '; 4 * 1024 * 1024 + 1]
        } else {
            notice(1700000000)
        };
        assert_eq!(watch.ingest(&frame).unwrap_err().code, "EVENT_STREAM_LIMIT");
    }
}

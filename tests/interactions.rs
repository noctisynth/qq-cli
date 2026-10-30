use serde_json::Value;
#[cfg(windows)]
use serde_json::json;
use std::process::Command;
fn run(args: &[&str], config: &std::path::Path) -> (i32, Value) {
    let o = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args(args)
        .arg("--config")
        .arg(config)
        .output()
        .unwrap();
    assert!(o.stderr.is_empty());
    (
        o.status.code().unwrap(),
        serde_json::from_slice(&o.stdout).unwrap(),
    )
}
#[test]
fn interactions_need_explicit_choices_and_poke_ids_stay_exact() {
    let t = tempfile::tempdir().unwrap();
    let config = t.path().join("missing.json");
    for args in [
        vec!["contact", "poke", "10002"],
        vec!["group", "poke", "20001", "10002"],
        vec!["contact", "input-status", "10002", "--event-type", "1"],
        vec![
            "message",
            "react",
            "synthetic-ref",
            "--emoji-id",
            "128",
            "--enabled",
            "false",
        ],
    ] {
        assert_eq!(run(&args, &config).0, 2);
    }
    for args in [
        vec!["contact", "poke", "10002", "--dry-run"],
        vec!["group", "poke", "20001", "10002", "--dry-run"],
        vec![
            "contact",
            "input-status",
            "10002",
            "--event-type",
            "1",
            "--dry-run",
        ],
    ] {
        let (c, v) = run(&args, &config);
        assert_eq!(c, 0, "{v}");
        assert_eq!(v["data"]["identityVerified"], false);
    }
    let (code, v) = run(
        &["contact", "poke", "9007199254740993", "--dry-run"],
        &config,
    );
    assert_eq!(code, 2);
    assert_eq!(v["error"]["code"], "POKE_ID_RANGE");
    assert!(!t.path().join("state").exists());
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_interactions_bind_sources_and_recover_write_results_without_repeating() {
    use qq_cli::{
        core::domain::Scope,
        messaging::message::Locator,
        storage::{
            credentials::{Credentials, SystemCredentials},
            references::References,
            state::{Installation, Instance, Store},
        },
    };
    use std::time::{Duration, Instant};
    if managed::child() {
        return;
    }
    for mode in [
        "react-set",
        "react-clear",
        "read",
        "read-long",
        "private-poke",
        "group-poke",
        "input",
        "fault",
        "mismatch",
        "source-denied",
        "read-denied",
        "long-id",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-interactions");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let credentials = Cleanup(SystemCredentials::new(store.root()).unwrap());
        credentials
            .0
            .set("synthetic-interactions", "synthetic-cli-token")
            .unwrap();
        let count = match mode {
            "source-denied" | "read-denied" | "long-id" => 0,
            "mismatch" => 10,
            "private-poke" | "group-poke" | "input" => 7,
            _ => 14,
        };
        let id = if matches!(mode, "read-long" | "long-id") {
            "4000000000"
        } else {
            "42"
        };
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let (endpoint, server) = http::serve(count, move |action, payload| {
            let data = match action {
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_msg" => {
                    assert_eq!(payload, &json!({"message_id":id}));
                    json!({"message_id":id,"message_type":"group","group_id":20001,"user_id":10002,"time":if mode=="mismatch"{101}else{100},"real_seq":7,"message":[{"type":"text","data":{"text":"synthetic-private-source"}}]})
                }
                _ => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let expected = match mode {
                        "private-poke" => "friend_poke",
                        "group-poke" => "group_poke",
                        "input" => "set_input_status",
                        "read" | "read-long" => "get_emoji_likes",
                        _ => "set_msg_emoji_like",
                    };
                    assert_eq!(action, expected);
                    let expected = match mode {
                        "private-poke" => json!({"user_id":"10002"}),
                        "group-poke" => json!({"group_id":"20001","user_id":"10002"}),
                        "input" => json!({"user_id":"10002","event_type":1.0}),
                        "read" | "read-long" => {
                            json!({"message_id":id,"group_id":"20001","emoji_id":"128","count":2.0})
                        }
                        _ => json!({"message_id":"42","emoji_id":"128","set":mode!="react-clear"}),
                    };
                    assert_eq!(payload, &expected);
                    if matches!(mode, "read" | "read-long") {
                        json!({"emoji_like_list":[{"user_id":"tiny-opaque","nick_name":"Synthetic","extra":"synthetic-native-secret"}]})
                    } else {
                        json!({"syntheticExtra":"synthetic-native-secret"})
                    }
                }
            };
            serde_json::to_vec(&json!({"status":if mode=="fault"&&action=="set_msg_emoji_like"{"failed"}else{"ok"},"retcode":if mode=="fault"&&action=="set_msg_emoji_like"{1200}else{0},"data":data})).unwrap()
        });
        let mut tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        tx.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: temp.path().join("unused"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-interactions".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let refs = References::open(
            &store,
            &credentials.0,
            Instant::now() + Duration::from_secs(5),
        )
        .unwrap();
        let token = refs
            .issue(
                "m1",
                &Locator {
                    scope: Scope {
                        instance_id: "instance-example".into(),
                        account: "10001".into(),
                    },
                    run_id: "run-example".into(),
                    conversation: "group:20001".parse().unwrap(),
                    backend_id: id.into(),
                    time_seconds: Some(100),
                    sequence: Some("7".into()),
                    sender_id: Some("10002".into()),
                },
            )
            .unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &credentials.0,
            "instance-example",
            "managed_interactions_bind_sources_and_recover_write_results_without_repeating",
        );
        let config = temp.path().join("config.json");
        let target = match mode {
            "private-poke" | "input" => "private:10002",
            "source-denied" => "group:20002",
            _ => "group:20001",
        };
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"permissions":{"targets":[target],"read":mode!="read-denied"}})).unwrap()).unwrap();
        let (path, mut args) = match mode {
            "private-poke" => ("contact.poke", vec!["contact", "poke", "10002"]),
            "group-poke" => ("group.poke", vec!["group", "poke", "20001", "10002"]),
            "input" => (
                "contact.input-status",
                vec!["contact", "input-status", "10002", "--event-type", "1"],
            ),
            "read" | "read-long" => (
                "message.reactions",
                vec![
                    "message",
                    "reactions",
                    &token,
                    "--emoji-id",
                    "128",
                    "--limit",
                    "2",
                ],
            ),
            _ => (
                "message.react",
                vec![
                    "message",
                    "react",
                    &token,
                    "--emoji-id",
                    "128",
                    "--enabled",
                    if mode == "react-clear" {
                        "false"
                    } else {
                        "true"
                    },
                ],
            ),
        };
        if !matches!(mode, "read" | "read-long" | "long-id") {
            let mut preview = args.clone();
            preview.push("--dry-run");
            let (c, v) = run(&preview, &config);
            assert_eq!(c, 0, "{mode}: {v}");
        }
        let read = matches!(mode, "read" | "read-long");
        if !read {
            args.extend(["--request-id", "interaction-example"]);
        }
        let server = server.start();
        let (c, v) = run(&args, &config);
        let success = !matches!(
            mode,
            "fault" | "mismatch" | "source-denied" | "read-denied" | "long-id"
        );
        assert_eq!(c, if success { 0 } else { 4 }, "{mode}: {v}");
        let schema = qq_cli::cli::registry::get(path).unwrap();
        assert!(
            jsonschema::validator_for(&schema["responseSchema"])
                .unwrap()
                .is_valid(&v),
            "{v}"
        );
        server.join().unwrap();
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(success || mode == "fault")
        );
        if read {
            assert_eq!(v["data"]["complete"], false);
            assert_eq!(v["data"]["items"][0]["reactorId"], "tiny-opaque");
            assert!(v["data"]["items"][0].get("extra").is_none());
        }
        if !read && (success || mode == "fault") {
            let (code, cached) = run(&args, &config);
            assert_eq!(code, c, "{cached}");
            if success {
                assert_eq!(cached["data"]["deduplicated"], true);
            }
        }
        fn scan(p: &std::path::Path) {
            for e in std::fs::read_dir(p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    scan(&p);
                } else if let Ok(s) = std::fs::read_to_string(p) {
                    assert!(!s.contains("synthetic-private-source"));
                    assert!(!s.contains("synthetic-native-secret"));
                }
            }
        }
        scan(store.root());
    }
}

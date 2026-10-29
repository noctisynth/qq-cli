use serde_json::{Value, json};
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
fn card_discovery_exposes_generation_and_exact_mini_app_input_shapes() {
    for name in [
        "ArkSharePeer",
        "ArkShareGroup",
        "send_ark_share",
        "send_group_ark_share",
        "get_mini_app_ark",
    ] {
        let d = qq_cli::transport::catalog::describe(&name.parse().unwrap());
        assert_eq!(serde_json::to_value(&d).unwrap()["effect"], "read");
        assert!(d.parameters_known);
    }
    let d = serde_json::to_value(qq_cli::transport::catalog::describe(
        &"get_mini_app_ark".parse().unwrap(),
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&d["parameterSchema"]).unwrap();
    let mut template = json!({"type":"bili","title":"Synthetic","desc":"example","picUrl":"https://invalid.invalid/image","jumpUrl":"https://invalid.invalid/page"});
    assert!(validator.is_valid(&template));
    template["rawArkData"] = json!(true);
    assert!(!validator.is_valid(&template));
    let custom = json!({"title":"Synthetic","desc":"example","picUrl":"","jumpUrl":"","iconUrl":"","appId":"synthetic-app","scene":"1","templateType":"1","businessType":"0","verType":"3","shareType":"0","versionId":"synthetic-version","sdkId":"synthetic-sdk","withShareTicket":"0","rawArkData":"true"});
    assert!(validator.is_valid(&custom));
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(
        run(
            &[
                "message",
                "click",
                "synthetic-ref",
                "--bot-app-id",
                "synthetic-app",
                "--button-id",
                "btn"
            ],
            &temp.path().join("missing.json")
        )
        .0,
        2
    );
}
#[cfg(windows)]
#[path = "common/http.rs"]
mod http;
#[cfg(windows)]
#[path = "common/managed.rs"]
mod managed;
#[cfg(windows)]
#[test]
fn managed_cards_are_reads_and_button_clicks_use_verified_sequences_once_only() {
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
        "contact",
        "group",
        "contact-failed",
        "empty",
        "click",
        "click-stale",
        "click-private",
        "click-no-seq",
        "click-denied",
        "contact-denied",
        "output-limit",
    ] {
        struct Cleanup(SystemCredentials);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-cards");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let t = tempfile::tempdir().unwrap();
        let store = Store::new(t.path().join("state"));
        let creds = Cleanup(SystemCredentials::new(store.root()).unwrap());
        creds
            .0
            .set("synthetic-cards", "synthetic-cli-token")
            .unwrap();
        let count = match mode {
            "click" => 14,
            "click-stale" => 10,
            "click-private" | "click-no-seq" | "click-denied" | "contact-denied" => 0,
            _ => 7,
        };
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let observed = calls.clone();
        let (endpoint, server) = http::serve(count, move |action, payload| {
            let data = match action {
                "get_status" => json!({"online":true}),
                "get_login_info" => json!({"user_id":"10001"}),
                "get_version_info" => json!({"app_version":"4.18.38"}),
                "get_msg" => {
                    json!({"message_id":42,"message_type":"group","group_id":20001,"user_id":10002,"time":100,"real_seq":if mode=="click-stale"{8}else{7},"message":[{"type":"text","data":{"text":"synthetic-private-body"}}]})
                }
                "ArkSharePeer" => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    assert_eq!(payload, &json!({"user_id":"10002","phone_number":""}));
                    json!({"result":if mode=="contact-failed"{17}else{0},"arkMsg":"{\"syntheticCard\":true}","unknown":"synthetic-native-secret"})
                }
                "ArkShareGroup" => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    assert_eq!(payload, &json!({"group_id":"20001"}));
                    json!(if mode == "output-limit" {
                        "x".repeat(910)
                    } else if mode == "empty" {
                        String::new()
                    } else {
                        "{\"syntheticCard\":true}".to_owned()
                    })
                }
                "click_inline_keyboard_button" => {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    assert_eq!(
                        payload,
                        &json!({"group_id":"20001","bot_appid":"synthetic-app","button_id":"btn","callback_data":"synthetic-private-callback","msg_seq":"7"})
                    );
                    Value::Null
                }
                _ => panic!("unexpected synthetic card action"),
            };
            serde_json::to_vec(&json!({"status":"ok","retcode":0,"data":data})).unwrap()
        });
        let mut tx = store
            .transaction(Instant::now() + Duration::from_secs(5))
            .unwrap();
        tx.registry.installations.push(Installation {
            id: "install-example".into(),
            version: "v4.18.38".into(),
            digest: "a".repeat(64),
            directory: t.path().join("unused"),
        });
        tx.registry.instances.push(Instance {
            id: "instance-example".into(),
            installation_id: "install-example".into(),
            endpoint,
            ws_endpoint: None,
            credential_key: "synthetic-cards".into(),
            account: Some("10001".into()),
            run_id: "run-example".into(),
            state: "ready".into(),
            process_id: None,
            process_started: None,
        });
        tx.commit().unwrap();
        let refs =
            References::open(&store, &creds.0, Instant::now() + Duration::from_secs(5)).unwrap();
        let token = refs
            .issue(
                "m1",
                &Locator {
                    scope: Scope {
                        instance_id: "instance-example".into(),
                        account: "10001".into(),
                    },
                    run_id: "run-example".into(),
                    conversation: if mode == "click-private" {
                        "private:10002"
                    } else {
                        "group:20001"
                    }
                    .parse()
                    .unwrap(),
                    backend_id: "42".into(),
                    time_seconds: Some(100),
                    sequence: if mode == "click-no-seq" {
                        None
                    } else {
                        Some("7".into())
                    },
                    sender_id: Some("10002".into()),
                },
            )
            .unwrap();
        let _owned = managed::Managed::attach(
            &store,
            &creds.0,
            "instance-example",
            "managed_cards_are_reads_and_button_clicks_use_verified_sequences_once_only",
        );
        let config = t.path().join("config.json");
        std::fs::write(&config,serde_json::to_vec(&json!({"stateDir":store.root(),"responseBytes":if mode=="output-limit"{1024}else{4194304},"permissions":{"targets":[if mode.ends_with("denied"){"group:20002"}else if mode.starts_with("contact"){"private:10002"}else{"group:20001"}]}})).unwrap()).unwrap();
        let (path, args) = if mode.starts_with("click") {
            (
                "message.click",
                vec![
                    "message",
                    "click",
                    &token,
                    "--bot-app-id",
                    "synthetic-app",
                    "--button-id",
                    "btn",
                    "--callback-data",
                    "synthetic-private-callback",
                    "--request-id",
                    "click-example",
                ],
            )
        } else if mode.starts_with("contact") {
            ("contact.card", vec!["contact", "card", "10002"])
        } else {
            ("group.card-json", vec!["group", "card-json", "20001"])
        };
        let server = server.start();
        let (c, v) = run(&args, &config);
        let success = matches!(mode, "contact" | "group" | "click");
        assert_eq!(
            c,
            if success {
                0
            } else if mode == "output-limit" {
                3
            } else {
                4
            },
            "{mode}: {v}"
        );
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
            usize::from(matches!(
                mode,
                "contact" | "group" | "contact-failed" | "empty" | "output-limit" | "click"
            ))
        );
        if mode == "click" {
            let (c, v) = run(&args, &config);
            assert_eq!(c, 0, "{v}");
            assert_eq!(v["data"]["deduplicated"], true);
        }
        fn scan(p: &std::path::Path) {
            for e in std::fs::read_dir(p).unwrap() {
                let p = e.unwrap().path();
                if p.is_dir() {
                    scan(&p);
                } else if let Ok(s) = std::fs::read_to_string(p) {
                    for secret in [
                        "synthetic-private-body",
                        "synthetic-native-secret",
                        "synthetic-private-callback",
                        "syntheticCard",
                    ] {
                        assert!(!s.contains(secret));
                    }
                }
            }
        }
        scan(store.root());
    }
}

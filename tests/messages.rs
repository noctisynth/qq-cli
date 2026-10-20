use std::cell::Cell;
use std::time::{Duration, Instant};

use qq_cli::core::domain::{Conversation, Scope};
use qq_cli::core::error::Result;
use qq_cli::messaging::message::{self, Locator};
use qq_cli::messaging::session::Session;
use qq_cli::storage::config::Config;
use qq_cli::storage::references::References;
use qq_cli::storage::state::Instance;
use qq_cli::transport::client::{Action, Backend};
use serde_json::{Value, json};

struct Fake {
    raw: Value,
    reads: Cell<usize>,
    identities: Cell<usize>,
    switch: bool,
}
impl Backend for Fake {
    fn call(&self, action: Action, _: &Value, _: Instant) -> Result<Value> {
        Ok(match action {
            Action::VersionInfo => json!({"app_version":"4.18.38"}),
            Action::Status => json!({"online":true}),
            Action::LoginInfo => {
                let n = self.identities.get() + 1;
                self.identities.set(n);
                json!({"user_id":if self.switch && n>1 {"10003"} else {"10001"}})
            }
            Action::Message => {
                self.reads.set(self.reads.get() + 1);
                self.raw.clone()
            }
            Action::Forward => {
                json!({"messages":[{"user_id":"10002","message":[{"type":"text","data":{"text":"forward sample"}}]}]})
            }
            _ => panic!("unexpected synthetic action"),
        })
    }
}
fn raw() -> Value {
    json!({"message_id":"9007199254740993","real_seq":"101","time":1700000000,
        "message_type":"group","group_id":"20001","self_id":"10001","user_id":"10002",
        "sender":{"user_id":"10002","nickname":"Sample"},"message":[
        {"type":"text","data":{"text":"synthetic body"}},
        {"type":"image","data":{"file":"sample.jpg","url":"https://invalid.example/private","file_size":"12"}},
        {"type":"forward","data":{"id":"synthetic-forward"}},
        {"type":"unsupported-kind","data":{"secret":"must not escape"}}]})
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
fn scope() -> Scope {
    Scope {
        instance_id: "instance-example".into(),
        account: "10001".into(),
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[test]
fn message_refs_keep_identity_without_body_and_expansion_uses_one_message_read() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [41; 32]);
    let message = message::normalize(
        &raw(),
        &scope(),
        "run-example",
        &Conversation::Group("20001".into()),
        &refs,
    )
    .unwrap();
    assert_eq!(message.value["id"], "9007199254740993");
    assert_eq!(message.text, "synthetic body");
    assert_eq!(message.value["segments"][1]["downloadable"], false);
    let serialized = serde_json::to_string(&message.value).unwrap();
    assert!(!serialized.contains("invalid.example"));
    assert!(!serialized.contains("must not escape"));
    for file in std::fs::read_dir(temp.path().join("refs")).unwrap() {
        let contents = std::fs::read_to_string(file.unwrap().path()).unwrap();
        assert!(!contents.contains("synthetic body"));
    }
    let backend = Fake {
        raw: raw(),
        reads: Cell::new(0),
        identities: Cell::new(0),
        switch: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    let result = message::get(
        message.value["messageRef"].as_str().unwrap(),
        true,
        &session,
        &refs,
        deadline(),
    )
    .unwrap();
    assert_eq!(backend.reads.get(), 1);
    let descriptor = qq_cli::cli::registry::get("message.get").unwrap();
    let validator = jsonschema::validator_for(&descriptor["responseSchema"]).unwrap();
    let envelope = serde_json::to_value(
        qq_cli::core::output::Envelope::success(result.clone()).scoped(scope()),
    )
    .unwrap();
    assert!(validator.is_valid(&envelope), "{envelope}");
    let mut invalid = envelope.clone();
    invalid["data"]["id"] = json!(40000);
    assert!(!validator.is_valid(&invalid));
    let mut invalid = envelope;
    invalid["data"]["rawBody"] = json!("must not leak");
    assert!(!validator.is_valid(&invalid));
    assert_eq!(result["segments"][2]["expanded"], true);
    assert_eq!(
        result["segments"][2]["nodes"][0]["segments"][0]["text"],
        "forward sample"
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("resourceId")
    );
}

#[test]
fn stale_runs_reused_message_ids_and_mid_read_account_changes_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [42; 32]);
    let initial = message::normalize(
        &raw(),
        &scope(),
        "run-example",
        &Conversation::Group("20001".into()),
        &refs,
    )
    .unwrap();
    let mut locator: Locator = refs
        .get("m1", initial.value["messageRef"].as_str().unwrap())
        .unwrap();
    let backend = Fake {
        raw: raw(),
        reads: Cell::new(0),
        identities: Cell::new(0),
        switch: false,
    };
    let config = Config::default();
    let instance = instance();
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    locator.run_id = "old-run".into();
    assert_eq!(
        message::fetch(&locator, &session, &refs, deadline())
            .unwrap_err()
            .code,
        "STALE_REFERENCE"
    );
    assert_eq!(backend.reads.get(), 0);
    locator.run_id = "run-example".into();
    locator.sequence = Some("100".into());
    assert_eq!(
        message::fetch(&locator, &session, &refs, deadline())
            .unwrap_err()
            .code,
        "STALE_REFERENCE"
    );
    let backend = Fake {
        raw: raw(),
        reads: Cell::new(0),
        identities: Cell::new(0),
        switch: true,
    };
    let session = Session {
        backend: &backend,
        config: &config,
        instance: &instance,
        expected_version: "4.18.38",
    };
    assert_eq!(
        message::get(
            initial.value["messageRef"].as_str().unwrap(),
            false,
            &session,
            &refs,
            deadline()
        )
        .unwrap_err()
        .code,
        "ACCOUNT_CHANGED"
    );
}

#[test]
fn malformed_peers_timestamps_and_unbounded_expansion_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [43; 32]);
    for (key, value) in [
        ("group_id", json!("20002")),
        ("time", json!(1700000000000_u64)),
        ("real_seq", json!(1.5)),
    ] {
        let mut invalid = raw();
        invalid[key] = value;
        assert!(
            message::normalize(
                &invalid,
                &scope(),
                "run-example",
                &Conversation::Group("20001".into()),
                &refs
            )
            .is_err()
        );
    }
    let mut invalid = raw();
    invalid["message"] = json!(vec![json!({"type":"text","data":{"text":"x"}}); 1001]);
    assert_eq!(
        message::normalize(
            &invalid,
            &scope(),
            "run-example",
            &Conversation::Group("20001".into()),
            &refs
        )
        .err()
        .unwrap()
        .code,
        "EXPANSION_LIMIT"
    );
}

#[test]
fn media_reference_is_revalidated_against_message_before_source_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [45; 32]);
    let initial = message::normalize(
        &raw(),
        &scope(),
        "run-example",
        &Conversation::Group("20001".into()),
        &refs,
    )
    .unwrap();
    let token = initial.value["segments"][1]["mediaRef"].as_str().unwrap();
    let instance = instance();
    for mode in ["ok", "changed_id", "changed_size", "denied", "account"] {
        let mut raw = raw();
        if mode == "changed_id" {
            raw["message"][1]["data"]["file"] = json!("different.jpg");
        }
        if mode == "changed_size" {
            raw["message"][1]["data"]["file_size"] = json!("13");
        }
        let backend = Fake {
            raw,
            reads: Cell::new(0),
            identities: Cell::new(0),
            switch: mode == "account",
        };
        let mut config = Config::default();
        config.permissions.media_download = mode != "denied";
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let result = qq_cli::messaging::media::verify_reference(token, &session, &refs, deadline());
        match mode {
            "ok" => assert_eq!(result.unwrap().file_id, "sample.jpg"),
            "denied" => {
                assert_eq!(result.unwrap_err().code, "PERMISSION_DENIED");
                assert_eq!(backend.identities.get(), 0);
            }
            "account" => assert_eq!(result.unwrap_err().code, "ACCOUNT_CHANGED"),
            _ => assert_eq!(result.unwrap_err().code, "MEDIA_REFERENCE_STALE"),
        }
        assert_eq!(backend.reads.get(), usize::from(mode != "denied"));
    }
}

#[test]
fn media_metadata_never_exposes_url_queries_or_unsafe_display_names() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [46; 32]);
    for file in [
        "https://invalid.example/image?token=synthetic-secret",
        "image#synthetic-secret",
        "bad\nname",
        "CON",
    ] {
        let mut raw = raw();
        raw["message"][1]["data"]["file"] = json!(file);
        let result = message::normalize(
            &raw,
            &scope(),
            "run-example",
            &Conversation::Group("20001".into()),
            &refs,
        )
        .unwrap();
        assert!(result.value["segments"][1].get("name").is_none());
        assert_eq!(result.value["segments"][1]["downloadable"], false);
        assert!(
            !serde_json::to_string(&result.value)
                .unwrap()
                .contains("synthetic-secret")
        );
    }
}

#[test]
fn media_sources_bind_element_positions_and_reject_changed_resources_or_legacy_ambiguity() {
    use qq_cli::messaging::message::MediaLocator;
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [92; 32]);
    let mut original = raw();
    original["message"] = json!([
        {"type":"image","data":{"file":"same.jpg","file_size":"12","url":"https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=first-resource&rkey=synthetic-private-key"}},
        {"type":"image","data":{"file":"same.jpg","file_size":"12","url":"https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=second-resource&rkey=synthetic-private-key"}}
    ]);
    let initial = message::normalize(
        &original,
        &scope(),
        "run-example",
        &Conversation::Group("20001".into()),
        &refs,
    )
    .unwrap();
    let token = initial.value["segments"][1]["mediaRef"].as_str().unwrap();
    let locator: MediaLocator = refs.get("f1", token).unwrap();
    assert_eq!(locator.segment_index, Some(1));
    assert!(locator.source_fingerprint.is_some());
    let instance = instance();
    let config = Config::default();
    for mode in [
        "same",
        "rotated",
        "changed",
        "swapped",
        "missing",
        "legacy",
        "legacy_unique",
    ] {
        let mut raw = original.clone();
        match mode {
            "rotated" => {
                raw["message"][1]["data"]["url"] = json!(
                    "https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=second-resource&rkey=rotated-private-key"
                )
            }
            "changed" => {
                raw["message"][1]["data"]["url"] = json!(
                    "https://multimedia.nt.qq.com.cn/download?appid=1407&fileid=different-resource&rkey=rotated-private-key"
                )
            }
            "swapped" => raw["message"].as_array_mut().unwrap().swap(0, 1),
            "legacy_unique" => {
                raw["message"].as_array_mut().unwrap().remove(0);
            }
            "missing" => {
                raw["message"][1]["data"]
                    .as_object_mut()
                    .unwrap()
                    .remove("url");
            }
            _ => (),
        }
        let backend = Fake {
            raw,
            reads: Cell::new(0),
            identities: Cell::new(0),
            switch: false,
        };
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let legacy = if mode.starts_with("legacy") {
            let mut locator = locator.clone();
            locator.segment_index = None;
            locator.source_fingerprint = None;
            Some(refs.issue("f1", &locator).unwrap())
        } else {
            None
        };
        let result = qq_cli::messaging::media::verify_reference(
            legacy.as_deref().unwrap_or(token),
            &session,
            &refs,
            deadline(),
        );
        if mode == "legacy_unique" {
            assert_eq!(result.unwrap().segment_index, Some(0));
        } else if matches!(mode, "same" | "rotated") {
            assert_eq!(result.unwrap().segment_index, Some(1));
        } else {
            assert_eq!(result.unwrap_err().code, "MEDIA_REFERENCE_STALE", "{mode}");
        }
        assert_eq!(backend.reads.get(), 1);
    }
    for entry in std::fs::read_dir(temp.path().join("refs")).unwrap() {
        let bytes = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(!bytes.contains("private-key"));
        assert!(!bytes.contains("https://"));
    }
}

#[test]
fn media_download_uses_scoped_bytes_and_withholds_output_when_guards_fail() {
    struct Download {
        base: Fake,
        mode: &'static str,
        identity: Cell<usize>,
        media_calls: Cell<usize>,
        trap: std::path::PathBuf,
    }
    impl Backend for Download {
        fn call(&self, action: Action, args: &Value, deadline: Instant) -> Result<Value> {
            match action {
                Action::LoginInfo => {
                    self.identity.set(self.identity.get() + 1);
                    Ok(
                        json!({"user_id":if self.mode=="guard" && self.identity.get()>=5 {"10003"}else{"10001"}}),
                    )
                }
                Action::Image | Action::File => {
                    self.media_calls.set(self.media_calls.get() + 1);
                    assert!(args.get("file").is_none());
                    assert!(matches!(
                        args["file_id"].as_str(),
                        Some(
                            "sample.jpg"
                                | "opaque-file-uuid"
                                | "/12345678-1234-5678-9abc-123456789abc"
                        )
                    ));
                    let mut data = json!({"base64":if self.mode=="malformed" {"%%%"}else{"YWJj"},"file_size":if self.mode=="size" {"4"}else{"3"},"file":self.trap,"url":"https://invalid.example/private"});
                    if self.mode == "unavailable" {
                        data.as_object_mut().unwrap().remove("base64");
                    }
                    Ok(data)
                }
                _ => self.base.call(action, args, deadline),
            }
        }
    }
    for mode in [
        "image",
        "file",
        "group_file",
        "guard",
        "malformed",
        "size",
        "unavailable",
        "existing",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let refs = References::with_key(temp.path().join("refs"), [92; 32]);
        let mut raw = raw();
        raw["message"][1]["data"]["file_size"] = json!("3");
        if mode != "image" {
            raw["message"][1]["type"] = json!("file");
            raw["message"][1]["data"]["file_id"] = json!(if mode == "group_file" {
                "/12345678-1234-5678-9abc-123456789abc"
            } else {
                "opaque-file-uuid"
            });
        }
        let initial = message::normalize(
            &raw,
            &scope(),
            "run-example",
            &Conversation::Group("20001".into()),
            &refs,
        )
        .unwrap();
        let token = initial.value["segments"][1]["mediaRef"].as_str().unwrap();
        let trap = temp.path().join("must-not-read.bin");
        std::fs::write(&trap, b"wrong backend path bytes").unwrap();
        let backend = Download {
            base: Fake {
                raw,
                reads: Cell::new(0),
                identities: Cell::new(0),
                switch: false,
            },
            mode,
            identity: Cell::new(0),
            media_calls: Cell::new(0),
            trap,
        };
        let config = Config::default();
        let instance = instance();
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let output = temp.path().join("output.bin");
        if mode == "existing" {
            std::fs::write(&output, b"existing output").unwrap();
        }
        let result =
            qq_cli::messaging::media::download(token, &output, &session, &refs, deadline());
        if matches!(mode, "file" | "group_file") {
            let result = serde_json::to_value(result.unwrap()).unwrap();
            assert_eq!(result["data"]["sizeBytes"], 3);
            assert_eq!(std::fs::read(output).unwrap(), b"abc");
            assert!(!result.to_string().contains("invalid.example"));
        } else {
            assert_eq!(
                result.err().unwrap().code,
                match mode {
                    "guard" => "ACCOUNT_CHANGED",
                    "malformed" => "MEDIA_BYTES_INVALID",
                    "size" => "MEDIA_CHANGED",
                    "existing" => "MEDIA_OUTPUT_INVALID",
                    "image" => "MEDIA_BINDING_UNVERIFIED",
                    _ => "MEDIA_BYTES_UNAVAILABLE",
                }
            );
            if mode == "existing" {
                assert_eq!(backend.base.reads.get(), 0);
                assert_eq!(std::fs::read(output).unwrap(), b"existing output");
            } else {
                assert!(!output.exists());
            }
            if mode == "image" {
                assert_eq!(backend.media_calls.get(), 0);
            }
        }
    }
}

#[test]
fn nested_forward_resources_expand_with_shared_cycle_depth_and_request_budgets() {
    struct Nested {
        base: Fake,
        mode: &'static str,
        calls: Cell<usize>,
    }
    impl Backend for Nested {
        fn call(&self, action: Action, args: &Value, deadline: Instant) -> Result<Value> {
            if !matches!(action, Action::Forward) {
                return self.base.call(action, args, deadline);
            }
            self.calls.set(self.calls.get() + 1);
            let id = args["id"].as_str().unwrap();
            let segments = match self.mode {
                "cycle" => json!([{"type":"forward","data":{"id":"root"}}]),
                "depth" => {
                    json!([{"type":"forward","data":{"id":format!("child-{}", self.calls.get())}}])
                }
                "requests" if id == "root" => json!(
                    (0..100)
                        .map(|n| json!({"type":"forward","data":{"id":format!("child-{n}")}}))
                        .collect::<Vec<_>>()
                ),
                "nested" if id == "root" => json!([{"type":"forward","data":{"id":"child"}}]),
                "bytes" => json!([{"type":"text","data":{"text":"x".repeat(4*1024*1024)}}]),
                _ => json!([{"type":"text","data":{"text":"nested sample"}}]),
            };
            Ok(json!({"messages":[{"user_id":"10002","message":segments}]}))
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [44; 32]);
    let config = Config::default();
    let instance = instance();
    for mode in ["nested", "inline", "cycle", "depth", "requests", "bytes"] {
        let mut raw = raw();
        raw["message"] = if mode == "inline" {
            json!([{"type":"forward","data":{"content":[{"user_id":"10002","message":[{"type":"forward","data":{"id":"child"}}]}]}}])
        } else {
            json!([{"type":"forward","data":{"id":"root"}}])
        };
        let initial = message::normalize(
            &raw,
            &scope(),
            "run-example",
            &Conversation::Group("20001".into()),
            &refs,
        )
        .unwrap();
        let backend = Nested {
            base: Fake {
                raw,
                reads: Cell::new(0),
                identities: Cell::new(0),
                switch: false,
            },
            mode,
            calls: Cell::new(0),
        };
        let session = Session {
            backend: &backend,
            config: &config,
            instance: &instance,
            expected_version: "4.18.38",
        };
        let result = message::get(
            initial.value["messageRef"].as_str().unwrap(),
            true,
            &session,
            &refs,
            deadline(),
        );
        if matches!(mode, "nested" | "inline") {
            let result = result.unwrap();
            assert_eq!(
                result["segments"][0]["nodes"][0]["segments"][0]["nodes"][0]["segments"][0]["text"],
                "nested sample"
            );
            assert!(
                !serde_json::to_string(&result)
                    .unwrap()
                    .contains("resourceId")
            );
            assert_eq!(backend.calls.get(), if mode == "nested" { 2 } else { 1 });
        } else {
            assert_eq!(result.err().unwrap().code, "EXPANSION_LIMIT", "{mode}");
        }
        assert!(backend.calls.get() <= 100);
        assert_eq!(backend.base.reads.get(), 1);
    }
}

#[test]
fn slash_prefixed_group_file_uuid_is_an_explicit_opaque_id_not_a_path() {
    let temp = tempfile::tempdir().unwrap();
    let refs = References::with_key(temp.path().join("refs"), [86; 32]);
    for (id, explicit, kind, allowed) in [
        ("/12345678-1234-5678-9abc-123456789abc", true, "file", true),
        (
            "/12345678-1234-5678-9abc-123456789abc",
            false,
            "file",
            false,
        ),
        (
            "/12345678-1234-5678-9abc-123456789abc",
            true,
            "image",
            false,
        ),
        (
            "//12345678-1234-5678-9abc-123456789abc",
            true,
            "file",
            false,
        ),
        (
            "/12345678-1234-5678-9abc-123456789abc/child",
            true,
            "file",
            false,
        ),
        ("/12345678-1234-5678-9abc-123456789abz", true, "file", false),
        ("/tmp/local-file", true, "file", false),
    ] {
        let mut raw = raw();
        raw["message"][1] =
            json!({"type":kind,"data":{"file":if explicit {"test.bin"}else{id},"file_size":"3"}});
        if explicit {
            raw["message"][1]["data"]["file_id"] = json!(id);
        }
        let normalized = message::normalize(
            &raw,
            &scope(),
            "run-example",
            &Conversation::Group("20001".into()),
            &refs,
        )
        .unwrap();
        let segment = &normalized.value["segments"][1];
        assert_eq!(segment.get("mediaRef").is_some(), allowed, "{id}");
        assert_eq!(segment["downloadable"], allowed, "{id}");
    }
    let mut private = raw();
    private["message_type"] = json!("private");
    private["target_id"] = json!("10001");
    private.as_object_mut().unwrap().remove("group_id");
    private["message"][1] = json!({"type":"file","data":{"file":"test.bin","file_id":"/12345678-1234-5678-9abc-123456789abc","file_size":"3"}});
    let normalized = message::normalize(
        &private,
        &scope(),
        "run-example",
        &Conversation::Private("10002".into()),
        &refs,
    )
    .unwrap();
    assert!(normalized.value["segments"][1].get("mediaRef").is_none());
    assert_eq!(normalized.value["segments"][1]["downloadable"], false);
}

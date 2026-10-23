use super::*;
use crate::messaging::watch::Source;
use crate::napcat::runtime::start_intent::{self, Begin, Target};
use crate::storage::{
    references::References,
    state::{Installation, atomic_json, private_dir},
};
use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
struct SigningOnly;
impl Credentials for SigningOnly {
    fn get(&self, key: &str) -> Result<String> {
        assert_eq!(key, "reference-signing-key");
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([46; 32]))
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("no credential write")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no credential deletion")
    }
}
struct Fixture {
    _temp: tempfile::TempDir,
    store: Store,
    refs: References,
    instance: Instance,
    record: Value,
}
impl Fixture {
    fn new(endpoint: String) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let refs = References::with_key(store.root().join("references"), [46; 32]);
        let mut instance = instance();
        instance.endpoint = endpoint;
        instance.state = "prepared".into();
        let installation = Installation {
            id: instance.installation_id.clone(),
            version: "synthetic".into(),
            digest: "a".repeat(64),
            directory: store.root().join("installation"),
        };
        let mut tx = store.transaction(deadline()).unwrap();
        tx.registry.instances.push(instance.clone());
        tx.registry.installations.push(installation.clone());
        tx.commit().unwrap();
        let target = Target {
            instance_id: instance.id.clone(),
            run_id: instance.run_id.clone(),
            installation_id: installation.id,
            installation_digest: installation.digest,
            input_hash: "b".repeat(64),
        };
        let Begin::Fresh(intent) =
            start_intent::begin(&store, &refs, "synthetic-start", &target, deadline()).unwrap()
        else {
            panic!("fresh")
        };
        intent.dispatch(|_| Ok(())).unwrap();
        instance = store.read().unwrap().instances.remove(0);
        let owner =
            crate::native::process::Pinned::observe(std::process::id(), deadline()).unwrap();
        let record = json!({"schemaVersion":1,"platform":"macos","requestId":"synthetic-start","target":target,"owner":owner.identity(),"endpoint":instance.endpoint,"wsEndpoint":instance.ws_endpoint});
        Self {
            _temp: temp,
            store,
            refs,
            instance,
            record,
        }
    }
    fn write(&self, kind: &str, value: &Value) -> String {
        let reference = self.refs.issue(kind, value).unwrap();
        let directory = self.store.root().join("macos-runs");
        private_dir(&directory).unwrap();
        let digest = Sha256::digest(
            serde_json::to_vec(&(&self.instance.id, &self.instance.run_id)).unwrap(),
        );
        atomic_json(
            &directory.join(format!("{digest:x}.json")),
            &json!({"schemaVersion":1,"reference":reference}),
        )
        .unwrap();
        reference
    }
    fn load(&self) -> Result<Context> {
        Context::load(
            &self.store,
            &self.store.read().unwrap(),
            &SigningOnly,
            &self.instance,
            deadline(),
        )
    }
}
#[test]
fn signed_dispatched_service_reaches_owned_http_transport() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let fixture = Fixture::new(format!("http://{}/", listener.local_addr().unwrap()));
    fixture.write("j2", &fixture.record);
    let context = fixture.load().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut byte = [0];
        while !bytes.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            bytes.push(byte[0]);
            assert!(bytes.len() < 8192);
        }
        let request = String::from_utf8(bytes).unwrap();
        assert!(request.starts_with("POST /get_status "));
        assert!(
            request
                .to_lowercase()
                .contains("authorization: bearer synthetic-token")
        );
        let length: usize = request
            .lines()
            .find_map(|line| {
                line.to_lowercase()
                    .strip_prefix("content-length:")
                    .map(|v| v.trim().parse().unwrap())
            })
            .unwrap();
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        let response = r#"{"status":"ok","retcode":0,"data":{"online":true}}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response.len(),
            response
        )
        .unwrap();
    });
    let config = Config::default();
    let client = context
        .client(&fixture.instance, "synthetic-token", &config)
        .unwrap();
    assert_eq!(
        client.call(Action::Status, &json!({}), deadline()).unwrap(),
        json!({"online":true})
    );
    server.join().unwrap();
    assert!(
        context
            .source(
                fixture.instance.ws_endpoint.as_deref().unwrap(),
                "synthetic-token"
            )
            .is_ok()
    );
}
#[test]
fn wrong_scope_platform_and_legacy_reference_never_recover() {
    let fixture = Fixture::new("http://127.0.0.1:1/".into());
    for (field, value) in [
        ("schemaVersion", json!(2)),
        ("platform", json!("windows")),
        ("endpoint", json!("http://127.0.0.1:3/")),
        ("wsEndpoint", json!("ws://127.0.0.1:3/")),
        ("requestId", json!("different-start")),
    ] {
        let mut record = fixture.record.clone();
        record[field] = value;
        fixture.write("j2", &record);
        assert!(fixture.load().is_err(), "accepted altered field {field}");
    }
    fixture.write("j1", &fixture.record);
    assert_eq!(fixture.load().err().unwrap().code, "REFERENCE_INVALID");
    let mut record = fixture.record.clone();
    record["target"]["inputHash"] = json!("c".repeat(64));
    fixture.write("j2", &record);
    assert_eq!(fixture.load().err().unwrap().code, "MACOS_RUN_CONFLICT");
}
#[test]
fn altered_identity_duplicate_registry_and_modified_bytes_are_rejected() {
    let fixture = Fixture::new("http://127.0.0.1:1/".into());
    let mut record = fixture.record.clone();
    record["owner"]["bootSession"] = json!("00000000-0000-0000-0000-000000000001");
    fixture.write("j2", &record);
    assert_eq!(
        fixture.load().err().unwrap().code,
        "PROCESS_IDENTITY_CHANGED"
    );
    let reference = fixture.write("j2", &fixture.record);
    let prefix = reference.rsplit_once('.').unwrap().0;
    std::fs::write(
        fixture
            .store
            .root()
            .join("references")
            .join(format!("{prefix}.json")),
        b"{}",
    )
    .unwrap();
    assert_eq!(fixture.load().err().unwrap().code, "REFERENCE_INVALID");
    fixture.write("j2", &fixture.record);
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry
        .installations
        .push(tx.registry.installations[0].clone());
    tx.commit().unwrap();
    assert_eq!(fixture.load().err().unwrap().code, "MACOS_RUN_CONFLICT");
}

#[test]
fn pending_or_unknown_dispatch_never_authorizes_service_recovery() {
    let fixture = Fixture::new("http://127.0.0.1:1/".into());
    fixture.write("j2", &fixture.record);
    for outcome in [
        start_intent::Outcome::Pending,
        start_intent::Outcome::Unknown,
    ] {
        let mut receipt =
            start_intent::get(&fixture.store, &fixture.refs, "synthetic-start").unwrap();
        receipt.outcome = outcome;
        let reference = fixture.refs.issue("s1", &receipt).unwrap();
        let digest = Sha256::digest(b"synthetic-start");
        atomic_json(
            &fixture
                .store
                .root()
                .join("start-requests")
                .join(format!("{digest:x}.json")),
            &json!({"schemaVersion":1,"reference":reference}),
        )
        .unwrap();
        assert_eq!(fixture.load().err().unwrap().code, "MACOS_RUN_CONFLICT");
    }
}

#[test]
fn signed_service_context_connects_websocket_with_header_authentication() {
    // Tungstenite requires its concrete HTTP error response in this callback signature.
    #[allow(clippy::result_large_err)]
    fn verify_upgrade(
        request: &tungstenite::handshake::server::Request,
        response: tungstenite::handshake::server::Response,
    ) -> std::result::Result<
        tungstenite::handshake::server::Response,
        tungstenite::handshake::server::ErrorResponse,
    > {
        assert_eq!(request.uri().path(), "/event");
        assert_eq!(
            request.headers().get("authorization").unwrap(),
            "Bearer synthetic-token"
        );
        Ok(response)
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let mut fixture = Fixture::new("http://127.0.0.1:1/".into());
    fixture.instance.ws_endpoint = Some(format!("ws://{}/event", listener.local_addr().unwrap()));
    fixture.record["wsEndpoint"] = json!(fixture.instance.ws_endpoint);
    let mut tx = fixture.store.transaction(deadline()).unwrap();
    tx.registry.instances[0] = fixture.instance.clone();
    tx.commit().unwrap();
    fixture.write("j2", &fixture.record);
    let context = fixture.load().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let _socket = tungstenite::accept_hdr(stream, verify_upgrade).unwrap();
    });
    let mut source = context
        .source(
            fixture.instance.ws_endpoint.as_deref().unwrap(),
            "synthetic-token",
        )
        .unwrap();
    source.connect(deadline()).unwrap();
    source.disconnect();
    server.join().unwrap();
}

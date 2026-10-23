use super::*;
use std::time::Duration;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "tests/macos.rs"]
mod macos;
struct NoCredentials;
impl Credentials for NoCredentials {
    fn get(&self, _: &str) -> Result<String> {
        panic!("no credential access")
    }
    fn set(&self, _: &str, _: &str) -> Result<()> {
        panic!("no credential writes")
    }
    fn delete(&self, _: &str) -> Result<()> {
        panic!("no credential deletion")
    }
}
fn instance() -> Instance {
    Instance {
        id: "original-instance".into(),
        run_id: "original-run".into(),
        installation_id: "unused-original".into(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: Some("ws://127.0.0.1:2/event".into()),
        credential_key: "original-synthetic".into(),
        account: None,
        state: "ready".into(),
        process_id: None,
        process_started: None,
    }
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
#[test]
fn missing_managed_record_refuses_before_credentials_or_state_creation() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let mut instance = instance();
    assert_eq!(
        Context::load(
            &store,
            &Registry::default(),
            &NoCredentials,
            &instance,
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "RUNTIME_RECORD_REQUIRED"
    );
    instance.credential_key =
        crate::napcat::runtime::runtime_store::api_credential_key(&instance.id, &instance.run_id)
            .unwrap();
    assert_eq!(
        Context::load(
            &store,
            &Registry::default(),
            &NoCredentials,
            &instance,
            deadline()
        )
        .err()
        .unwrap()
        .code,
        "RUNTIME_RECORD_REQUIRED"
    );
    assert!(!store.root().exists());
}
#[test]
fn transport_context_cannot_be_reused_for_another_instance_or_event_endpoint() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let instance = instance();
    let context = Context {
        instance: instance.clone(),
        #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
        service: None,
    };
    let mut changed = instance.clone();
    changed.run_id = "other-run".into();
    assert_eq!(
        context
            .client(&changed, "original-token", &Config::default())
            .err()
            .unwrap()
            .code,
        "TRANSPORT_CONTEXT_MISMATCH"
    );
    assert_eq!(
        context
            .source("ws://127.0.0.1:3/event", "original-token")
            .err()
            .unwrap()
            .code,
        "TRANSPORT_CONTEXT_MISMATCH"
    );
    assert!(!store.root().exists());
}

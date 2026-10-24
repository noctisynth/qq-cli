use super::*;
use crate::{
    napcat::install::catalog,
    storage::{references::References, state::Store},
};
use std::time::Duration;
#[test]
fn ready_label_and_open_endpoint_without_original_inputs_never_probe_or_create_evidence() {
    let trap = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().canonicalize().unwrap().join("state"));
    let refs = References::with_key(store.root().join("references"), [86; 32]);
    let release = catalog::lookup("4.18.41").unwrap();
    let instance = Instance {
        id: "instance".into(),
        run_id: "run".into(),
        installation_id: "install".into(),
        endpoint: format!("http://{}/", trap.local_addr().unwrap()),
        ws_endpoint: None,
        credential_key: "must-not-read".into(),
        account: Some("10001".into()),
        state: "ready".into(),
        process_id: None,
        process_started: None,
    };
    let installed = Installation {
        id: "install".into(),
        version: release.version.into(),
        digest: release.archive.sha256.into(),
        directory: store.root().join("missing-installation"),
    };
    let end = Instant::now() + Duration::from_secs(10);
    let mut tx = store.transaction(end).unwrap();
    tx.registry.instances.push(instance.clone());
    tx.registry.installations.push(installed.clone());
    tx.commit().unwrap();
    struct Never;
    impl Credentials for Never {
        fn get(&self, _: &str) -> Result<String> {
            panic!("credentials must follow original-input authentication")
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            panic!("no writes")
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("no deletion")
        }
    }
    let credentials = Never;
    let context = start::Context {
        store: &store,
        references: &refs,
        credentials: &credentials,
    };
    assert!(record(&context, &Config::default(), &instance, end).is_err());
    assert!(!local::present(&store).unwrap());
    assert!(!store.root().join("references").exists());
    assert_eq!(
        trap.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        record(
            &context,
            &Config::default(),
            &instance,
            Instant::now() - Duration::from_secs(1)
        )
        .unwrap_err()
        .code,
        "DEADLINE"
    );
    let mut info = SessionInfo {
        account: "10001".into(),
        nickname: None,
        version: "4.18.41".into(),
        online: true,
    };
    require_online(&info, &instance, &installed).unwrap();
    info.account = "10002".into();
    assert!(require_online(&info, &instance, &installed).is_err());
    info.account = "10001".into();
    info.online = false;
    assert!(require_online(&info, &instance, &installed).is_err());
    info.online = true;
    info.version = "4.18.38".into();
    assert!(require_online(&info, &instance, &installed).is_err());
    struct Unreviewed;
    impl Backend for Unreviewed {
        fn call(
            &self,
            action: crate::transport::client::Action,
            _: &serde_json::Value,
            _: Instant,
        ) -> Result<serde_json::Value> {
            if matches!(action, crate::transport::client::Action::VersionInfo) {
                return Ok(serde_json::json!({"app_version":"unreviewed"}));
            }
            panic!("unreviewed version must stop before account or status reads");
        }
    }
    assert!(inspect_online(&Unreviewed, &installed, end).is_err());
}

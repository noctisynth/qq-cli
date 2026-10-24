use super::*;
use crate::napcat::runtime::{
    macos_layout::{self, Preparation},
    runtime_config::Ports,
};
use crate::storage::{
    references::References,
    state::{Installation, Instance, Store},
};
use std::{os::unix::fs::PermissionsExt, time::Duration};
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn setup() -> (tempfile::TempDir, Store, References) {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let store = Store::new(base.join("state"));
    let mut transaction = store.transaction(deadline()).unwrap();
    transaction.registry.installations.push(Installation {
        id: "install".into(),
        version: "synthetic".into(),
        digest: "a".repeat(64),
        directory: base.join("framework"),
    });
    transaction.registry.instances.push(Instance {
        id: "instance".into(),
        installation_id: "install".into(),
        endpoint: "http://127.0.0.1:24001".into(),
        ws_endpoint: None,
        credential_key: "synthetic".into(),
        account: None,
        run_id: "run".into(),
        state: "prepared".into(),
        process_id: None,
        process_started: None,
    });
    transaction.commit().unwrap();
    let references = References::with_key(store.root().join("references"), [17; 32]);
    (temp, store, references)
}

fn fixture() -> (tempfile::TempDir, Store, Layout, Bundle) {
    let (temp, store, refs) = setup();
    let Preparation::Ready(layout) =
        macos_layout::prepare(&store, &refs, "instance", "layout", deadline()).unwrap()
    else {
        panic!("not ready")
    };
    let bundle = Bundle::generate(
        "4.18.41",
        Ports {
            http: 24001,
            events: 24002,
            control: 24003,
        },
    )
    .unwrap();
    (temp, store, layout, bundle)
}
#[test]
fn config_matches_workdir_and_roundtrips_without_secret_receipts() {
    let (_temp, _store, layout, bundle) = fixture();
    let published = publish(&layout, &bundle, deadline(), || Ok(())).unwrap();
    published.verify(&layout, &bundle, deadline()).unwrap();
    let bytes = serde_json::to_vec(&published).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(!text.contains(bundle.api_token()));
    assert!(!text.contains(bundle.control_token()));
    assert!(!text.contains(bundle.jwt_secret()));
    let recovered: Published = serde_json::from_slice(&bytes).unwrap();
    recovered.verify(&layout, &bundle, deadline()).unwrap();
    assert!(
        std::fs::read_dir(layout.directory().join("config"))
            .unwrap()
            .next()
            .is_none()
    );
    let config = layout.directory().join("napcat-data/config");
    let web: serde_json::Value =
        serde_json::from_slice(&std::fs::read(config.join("webui.json")).unwrap()).unwrap();
    assert_eq!(web["host"], "127.0.0.1");
    assert_eq!(web["autoLoginAccount"], "");
    assert!(publish(&layout, &bundle, deadline(), || Ok(())).is_err());
}
#[test]
fn extra_account_configuration_is_preserved_before_any_write() {
    let (_temp, _store, layout, bundle) = fixture();
    let path = layout
        .directory()
        .join("napcat-data/config/onebot11_synthetic.json");
    std::fs::write(&path, b"preserve").unwrap();
    assert!(publish(&layout, &bundle, deadline(), || Ok(())).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
}
#[test]
fn changed_bytes_modes_hardlinks_and_replacement_are_rejected() {
    for mode in 0..4 {
        let (_temp, _store, layout, bundle) = fixture();
        let published = publish(&layout, &bundle, deadline(), || Ok(())).unwrap();
        let path = layout.directory().join("napcat-data/config/webui.json");
        match mode {
            0 => std::fs::write(&path, b"changed").unwrap(),
            1 => std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap(),
            2 => std::fs::hard_link(&path, layout.directory().join("alias")).unwrap(),
            _ => {
                let bytes = std::fs::read(&path).unwrap();
                std::fs::rename(&path, layout.directory().join("old-webui")).unwrap();
                std::fs::write(&path, bytes).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
        assert!(published.verify(&layout, &bundle, deadline()).is_err());
    }
}
#[test]
fn guard_failure_retains_partial_files_and_directory_exchange_stops_writes() {
    for swap in [false, true] {
        let (_temp, _store, layout, bundle) = fixture();
        let path = layout.directory().join("napcat-data/config");
        let calls = std::cell::Cell::new(0);
        let result = publish(&layout, &bundle, deadline(), || {
            calls.set(calls.get() + 1);
            if calls.get() == 3 {
                if swap {
                    std::fs::rename(&path, path.with_file_name("old-config")).unwrap();
                    std::fs::create_dir(&path).unwrap();
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                        .unwrap();
                } else {
                    return Err(invalid());
                }
            }
            Ok(())
        });
        assert!(result.is_err());
        if swap {
            assert_eq!(std::fs::read_dir(&path).unwrap().count(), 0);
            assert_eq!(
                std::fs::read_dir(path.with_file_name("old-config"))
                    .unwrap()
                    .count(),
                1
            );
        } else {
            assert_eq!(std::fs::read_dir(&path).unwrap().count(), 1);
            assert!(publish(&layout, &bundle, deadline(), || Ok(())).is_err());
        }
    }
}

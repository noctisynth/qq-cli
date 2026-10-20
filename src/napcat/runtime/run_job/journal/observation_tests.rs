use super::*;
use crate::storage::state::{Installation, Instance};
use std::time::{Duration, Instant};

#[test]
fn observed_input_hash_cannot_promote_a_pending_dispatch_or_missing_service() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [81; 32]);
    let deadline = Instant::now() + Duration::from_secs(5);
    let job = crate::native::job::Job::create(deadline).unwrap();
    let instance = Instance {
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
    };
    let installation = Installation {
        id: instance.installation_id.clone(),
        version: "v4.18.38".into(),
        digest: "a".repeat(64),
        directory: temp.path().join("install"),
    };
    let target = crate::napcat::runtime::start_intent::Target {
        instance_id: instance.id.clone(),
        run_id: instance.run_id.clone(),
        installation_id: installation.id.clone(),
        installation_digest: installation.digest.clone(),
        input_hash: "b".repeat(64),
    };
    let record = Record {
        schema_version: 1,
        request_id: "synthetic-start".into(),
        target: target.clone(),
        job: job.id().clone(),
        outcome: Outcome::Pending,
        root: None,
        service: None,
        discovery: None,
        prelogin: None,
    };
    save(
        &path(&store, &instance.id, &instance.run_id).unwrap(),
        &refs,
        &record,
    )
    .unwrap();
    for hash in ["c".repeat(64), target.input_hash] {
        assert_eq!(
            observed_target(
                &store,
                &refs,
                &instance,
                &installation,
                &temp.path().join("QQ.exe"),
                &[hash],
                deadline
            )
            .unwrap_err()
            .code,
            "RUN_JOB_CONFLICT"
        );
    }
    assert!(!store.root().join("compatibility-evidence").exists());
}

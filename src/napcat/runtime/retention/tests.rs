use super::*;
use crate::storage::state::{Instance, Store};
use std::time::Duration;

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
fn binding<'a>(job: &'a Job, owner: &'a Pinned) -> Binding<'a> {
    Binding {
        instance: "original-instance",
        run: "original-run",
        job: job.id(),
        target: owner.identity(),
    }
}
fn setup(tx: &mut Transaction<'_>) {
    tx.registry.instances.push(Instance {
        id: "original-instance".into(),
        run_id: "original-run".into(),
        installation_id: "unused-original".into(),
        endpoint: "http://127.0.0.1:1/".into(),
        ws_endpoint: None,
        credential_key: "unused-original".into(),
        account: None,
        state: "starting".into(),
        process_id: None,
        process_started: None,
    });
    tx.persist().unwrap();
}
// Journal-only fixture: this numeric value is never passed to any native API.
fn receipt(job: &Job, owner: &Pinned) -> Retained {
    serde_json::from_value(serde_json::json!({"schemaVersion":1,"job":job.id(),
            "target":owner.identity(),"remoteHandle":4}))
    .unwrap()
}
fn journal(store: &Store) -> std::path::PathBuf {
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&("original-instance", "original-run")).unwrap())
    );
    store
        .root()
        .join("retentions")
        .join(format!("{digest}.json"))
}
#[test]
fn pending_precedes_operation_and_cached_result_cannot_change_target() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [42; 32]);
    let mut tx = store.transaction(deadline()).unwrap();
    setup(&mut tx);
    let job = Job::create(deadline()).unwrap();
    let owner = Pinned::observe(std::process::id(), deadline()).unwrap();
    let report = once_with(&tx, &refs, binding(&job, &owner), deadline(), || {
        let pointer: Pointer = read_json(&journal(&store), 4096)?;
        let record: Record = refs.get("t1", &pointer.reference)?;
        assert_eq!(record.outcome, Outcome::Pending);
        assert!(record.receipt.is_none());
        Ok(receipt(&job, &owner))
    })
    .unwrap();
    assert_eq!(report.outcome(), Outcome::Retained);
    tx.registry.instances[0].state = "stopped".into();
    tx.persist().unwrap();
    assert_eq!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || panic!(
            "no replay"
        ))
        .unwrap()
        .outcome(),
        Outcome::Retained
    );
    let other = Job::create(deadline()).unwrap();
    assert_eq!(
        once_with(&tx, &refs, binding(&other, &owner), deadline(), || panic!(
            "no rebind"
        ))
        .err()
        .unwrap()
        .code,
        "RETENTION_CONFLICT"
    );
    let wrong = References::with_key(store.root().join("references"), [43; 32]);
    let before = std::fs::read(journal(&store)).unwrap();
    assert!(
        once_with(&tx, &wrong, binding(&job, &owner), deadline(), || panic!(
            "no fallback"
        ))
        .is_err()
    );
    assert_eq!(std::fs::read(journal(&store)).unwrap(), before);
}
#[test]
fn interrupted_final_write_becomes_unknown_without_repeating_operation() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [44; 32]);
    let mut tx = store.transaction(deadline()).unwrap();
    setup(&mut tx);
    let job = Job::create(deadline()).unwrap();
    let owner = Pinned::observe(std::process::id(), deadline()).unwrap();
    let directory = store.root().join("retentions");
    let saved = store.root().join("original-saved-retentions");
    assert!(directory.starts_with(temp.path()) && saved.starts_with(temp.path()));
    let result = once_with(&tx, &refs, binding(&job, &owner), deadline(), || {
        std::fs::rename(&directory, &saved).unwrap();
        std::fs::write(&directory, b"original-obstruction").unwrap();
        Ok(receipt(&job, &owner))
    });
    assert_eq!(result.err().unwrap().code, "RETENTION_OUTCOME_UNKNOWN");
    assert_eq!(std::fs::read(&directory).unwrap(), b"original-obstruction");
    std::fs::remove_file(&directory).unwrap();
    std::fs::rename(&saved, &directory).unwrap();
    assert_eq!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || panic!(
            "never retry pending"
        ))
        .unwrap()
        .outcome(),
        Outcome::Unknown
    );
    assert_eq!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || panic!(
            "never retry unknown"
        ))
        .unwrap()
        .outcome(),
        Outcome::Unknown
    );
}
#[test]
fn inactive_or_expired_requests_do_not_dispatch_and_failed_operation_is_not_retried() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::new(temp.path().join("state"));
    let refs = References::with_key(store.root().join("references"), [45; 32]);
    let mut tx = store.transaction(deadline()).unwrap();
    setup(&mut tx);
    let job = Job::create(deadline()).unwrap();
    let owner = Pinned::observe(std::process::id(), deadline()).unwrap();
    tx.registry.instances[0].state = "prepared".into();
    assert!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || panic!(
            "inactive"
        ))
        .is_err()
    );
    assert!(
        once_with(
            &tx,
            &refs,
            binding(&job, &owner),
            Instant::now(),
            || panic!("expired")
        )
        .is_err()
    );
    assert!(!journal(&store).exists());
    tx.registry.instances[0].state = "starting".into();
    assert_eq!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || Err(
            AppError::storage()
        ))
        .unwrap()
        .outcome(),
        Outcome::Unknown
    );
    assert_eq!(
        once_with(&tx, &refs, binding(&job, &owner), deadline(), || panic!(
            "failed once"
        ))
        .unwrap()
        .outcome(),
        Outcome::Unknown
    );
}

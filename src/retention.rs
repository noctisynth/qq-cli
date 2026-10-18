//! At-most-once job retention inside an existing state transaction, never a nested lock.
use crate::config::read_json;
use crate::error::{AppError, Result};
use crate::native_job::{Job, JobId, Retained};
use crate::process::{Identity, Pinned};
use crate::references::References;
use crate::state::{Transaction, atomic_json, private_dir, reject_links};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Retained,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    instance_id: String,
    run_id: String,
    job: JobId,
    target: Identity,
    outcome: Outcome,
    receipt: Option<Retained>,
}
/// Historical operation result, NOT proof the target still owns its handle.
/// Intentionally not Debug/Serialize; private receipt must not enter public output.
pub struct Report {
    outcome: Outcome,
    receipt: Option<Retained>,
}
impl Report {
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
    pub fn receipt(&self) -> Option<&Retained> {
        self.receipt.as_ref()
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub fn retain_once(
    transaction: &Transaction<'_>,
    references: &References,
    instance: &str,
    run: &str,
    job: &Job,
    owner: &Pinned,
    deadline: Instant,
) -> Result<Report> {
    once_with(
        transaction,
        references,
        Binding {
            instance,
            run,
            job: job.id(),
            target: owner.identity(),
        },
        deadline,
        || job.retain_in(owner, deadline),
    )
}
struct Binding<'a> {
    instance: &'a str,
    run: &'a str,
    job: &'a JobId,
    target: &'a Identity,
}
/// Read an established retention; a missing log must never trigger another native copy.
pub(crate) fn existing(
    transaction: &Transaction<'_>,
    references: &References,
    instance: &str,
    run: &str,
    job: &Job,
    owner: &Pinned,
    deadline: Instant,
) -> Result<Report> {
    access(
        transaction,
        references,
        Binding {
            instance,
            run,
            job: job.id(),
            target: owner.identity(),
        },
        deadline,
        None::<fn() -> Result<Retained>>,
    )
}
fn once_with(
    transaction: &Transaction<'_>,
    references: &References,
    binding: Binding<'_>,
    deadline: Instant,
    operation: impl FnOnce() -> Result<Retained>,
) -> Result<Report> {
    access(transaction, references, binding, deadline, Some(operation))
}
fn access(
    transaction: &Transaction<'_>,
    references: &References,
    binding: Binding<'_>,
    deadline: Instant,
    operation: Option<impl FnOnce() -> Result<Retained>>,
) -> Result<Report> {
    let Binding {
        instance,
        run,
        job,
        target,
    } = binding;
    crate::deadline::check(deadline)?;
    crate::domain::request_key(instance).map_err(|_| AppError::invalid())?;
    crate::domain::request_key(run).map_err(|_| AppError::invalid())?;
    job.validate()?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(instance, run)).map_err(|_| AppError::storage())?)
    );
    let path = transaction
        .store()
        .root()
        .join("retentions")
        .join(format!("{digest}.json"));
    reject_links(&path)?;
    match read_json::<Pointer>(&path, 4096) {
        Ok(pointer) => {
            if pointer.schema_version != 1 {
                return Err(AppError::storage());
            }
            let mut record: Record = references.get("t1", &pointer.reference)?;
            if record.schema_version != 1 {
                return Err(AppError::storage());
            }
            if record.instance_id != instance
                || record.run_id != run
                || &record.job != job
                || &record.target != target
            {
                return Err(conflict());
            }
            match (record.outcome, &record.receipt) {
                (Outcome::Retained, Some(receipt)) => receipt.validate_for(job, target)?,
                (Outcome::Pending | Outcome::Unknown, None) => (),
                _ => return Err(AppError::storage()),
            }
            if record.outcome == Outcome::Pending {
                record.outcome = Outcome::Unknown;
                save(&path, references, &record)?;
            }
            return Ok(Report {
                outcome: record.outcome,
                receipt: record.receipt,
            });
        }
        Err(e) if e.code == "FILE_NOT_FOUND" => (),
        Err(e) => return Err(e),
    }
    let operation = operation.ok_or_else(conflict)?;
    let registered = transaction
        .registry
        .instances
        .iter()
        .find(|i| i.id == instance && i.run_id == run)
        .ok_or_else(conflict)?;
    if !matches!(
        registered.state.as_str(),
        "starting" | "login_required" | "ready"
    ) {
        return Err(conflict());
    }
    let mut record = Record {
        schema_version: 1,
        instance_id: instance.into(),
        run_id: run.into(),
        job: job.clone(),
        target: target.clone(),
        outcome: Outcome::Pending,
        receipt: None,
    };
    save(&path, references, &record)?;
    crate::deadline::check(deadline)?;
    if let Ok(receipt) = operation()
        && receipt.validate_for(job, target).is_ok()
    {
        record.outcome = Outcome::Retained;
        record.receipt = Some(receipt);
    } else {
        record.outcome = Outcome::Unknown;
    }
    save(&path, references, &record).map_err(|_| unknown())?;
    Ok(Report {
        outcome: record.outcome,
        receipt: record.receipt,
    })
}
fn save(path: &Path, references: &References, record: &Record) -> Result<()> {
    reject_links(path)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    let reference = references.issue("t1", record)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}
fn conflict() -> AppError {
    AppError::new(
        "RETENTION_CONFLICT",
        "This run's retention target or active phase no longer matches; no new handle was retained.",
        4,
    )
}
fn unknown() -> AppError {
    let mut error = AppError::new(
        "RETENTION_OUTCOME_UNKNOWN",
        "Retention may have occurred but its receipt could not be persisted; do not repeat it automatically.",
        5,
    );
    error.outcome = Some("unknown");
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Instance, Store};
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
}

//! At-most-once native-launch dispatch journal. This is not a startup trust gate.
//! Callers must verify license, installation, prepared material and compatibility
//! under the same state lock before dispatch. Native process APIs belong to the caller.
use crate::core::error::{AppError, Result};
use crate::storage::config::read_json;
use crate::storage::references::References;
use crate::storage::state::{Store, Transaction, atomic_json, private_dir, reject_links};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Target {
    pub instance_id: String,
    pub run_id: String,
    pub installation_id: String,
    pub installation_digest: String,
    /// Hash of the complete verified launch inputs, never raw credentials or paths.
    pub input_hash: String,
}
pub use crate::core::contracts::write::StartOutcome as Outcome;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Record {
    pub schema_version: u8,
    pub request_id: String,
    pub target: Target,
    pub outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub enum Begin<'a> {
    Existing(Record),
    Fresh(Intent<'a>),
}
pub struct Intent<'a> {
    transaction: Transaction<'a>,
    references: &'a References,
    path: PathBuf,
    record: Record,
    deadline: Instant,
}
fn path(store: &Store, key: &str) -> Result<PathBuf> {
    crate::core::domain::request_key(key).map_err(|_| AppError::invalid())?;
    Ok(store
        .root()
        .join("start-requests")
        .join(format!("{:x}.json", Sha256::digest(key.as_bytes()))))
}
fn run_path(store: &Store, target: &Target) -> Result<PathBuf> {
    run_path_for(store, &target.instance_id, &target.run_id)
}
fn run_path_for(store: &Store, instance_id: &str, run_id: &str) -> Result<PathBuf> {
    for key in [instance_id, run_id] {
        crate::core::domain::request_key(key).map_err(|_| AppError::storage())?;
    }
    let bytes = serde_json::to_vec(&(instance_id, run_id)).map_err(|_| AppError::storage())?;
    Ok(store
        .root()
        .join("start-runs")
        .join(format!("{:x}.json", Sha256::digest(bytes))))
}
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) fn has_run_claim(store: &Store, instance_id: &str, run_id: &str) -> Result<bool> {
    let path = run_path_for(store, instance_id, run_id)?;
    crate::storage::state::reject_links(&path)?;
    path.try_exists().map_err(|_| AppError::storage())
}
fn validate(target: &Target) -> Result<()> {
    for key in [&target.instance_id, &target.run_id, &target.installation_id] {
        crate::core::domain::request_key(key).map_err(|_| AppError::storage())?;
    }
    for hash in [&target.installation_digest, &target.input_hash] {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(AppError::storage());
        }
    }
    Ok(())
}
/// Read-only: does not create state or inspect any process, socket or credential.
pub fn get(store: &Store, references: &References, key: &str) -> Result<Record> {
    let path = path(store, key)?;
    let record = load(&path, references)?;
    if record.request_id != key {
        return Err(AppError::storage());
    }
    Ok(record)
}
fn load(path: &std::path::Path, references: &References) -> Result<Record> {
    reject_links(path)?;
    let pointer: Pointer = read_json(path, 4096).map_err(|e| {
        if e.code == "FILE_NOT_FOUND" {
            AppError::new(
                "REQUEST_NOT_FOUND",
                "No startup intent exists for this key.",
                4,
            )
        } else {
            e
        }
    })?;
    if pointer.schema_version != 1 {
        return Err(AppError::storage());
    }
    let record: Record = references.get("s1", &pointer.reference)?;
    validate(&record.target)?;
    crate::core::domain::request_key(&record.request_id).map_err(|_| AppError::storage())?;
    if record.schema_version != 1 {
        return Err(AppError::storage());
    }
    Ok(record)
}
fn save(path: &std::path::Path, references: &References, record: &Record) -> Result<()> {
    reject_links(path)?;
    private_dir(path.parent().ok_or_else(AppError::storage)?)?;
    let reference = references.issue("s1", record)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}
pub fn begin<'a>(
    store: &'a Store,
    references: &'a References,
    key: &str,
    target: &Target,
    deadline: Instant,
) -> Result<Begin<'a>> {
    crate::core::deadline::check(deadline)?;
    validate(target).map_err(|_| AppError::invalid())?;
    let path = path(store, key)?;
    let run_path = run_path(store, target)?;
    let mut transaction = store.transaction(deadline)?;
    crate::requests::keys::guard_start(store, key)?;
    crate::core::deadline::check(deadline)?;
    match get(store, references, key) {
        Ok(mut record) => {
            if &record.target != target {
                return Err(conflict());
            }
            let claim = load(&run_path, references).map_err(|_| AppError::storage())?;
            check_claim(&claim, key, target)?;
            if record.outcome == Outcome::Pending {
                record.outcome = Outcome::Unknown;
                save(&path, references, &record)?;
            }
            return Ok(Begin::Existing(record));
        }
        Err(e) if e.code == "REQUEST_NOT_FOUND" => (),
        Err(e) => return Err(e),
    }
    // A run claim precedes both the request index and registry commit. Even a
    // different key cannot replay a run after a crash between those writes.
    match load(&run_path, references) {
        Ok(mut claim) => {
            check_claim(&claim, key, target)?;
            claim.outcome = Outcome::Unknown;
            save(&path, references, &claim)?;
            return Ok(Begin::Existing(claim));
        }
        Err(e) if e.code == "REQUEST_NOT_FOUND" => (),
        Err(e) => return Err(e),
    }
    let installation = transaction
        .registry
        .installations
        .iter()
        .find(|i| i.id == target.installation_id && i.digest == target.installation_digest)
        .ok_or_else(conflict)?;
    let installation_id = installation.id.clone();
    let instance = transaction
        .registry
        .instances
        .iter_mut()
        .find(|i| {
            i.id == target.instance_id
                && i.run_id == target.run_id
                && i.installation_id == installation_id
        })
        .ok_or_else(conflict)?;
    if instance.state != "prepared"
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
    {
        return Err(conflict());
    }
    let record = Record {
        schema_version: 1,
        request_id: key.into(),
        target: target.clone(),
        outcome: Outcome::Pending,
    };
    // A durable intent MUST precede registry mutation and every dispatch callback.
    save(&run_path, references, &record)?;
    save(&path, references, &record)?;
    instance.state = "starting".into();
    transaction.persist()?;
    Ok(Begin::Fresh(Intent {
        transaction,
        references,
        path,
        record,
        deadline,
    }))
}
fn check_claim(claim: &Record, key: &str, target: &Target) -> Result<()> {
    if claim.outcome != Outcome::Pending {
        return Err(AppError::storage());
    }
    if claim.request_id != key || &claim.target != target {
        return Err(conflict());
    }
    Ok(())
}
impl Intent<'_> {
    /// The closure receives the locked registry for final startup gates. An error
    /// is conservatively unknown: a failed return may follow a successful spawn.
    /// Success means dispatch returned, not that QQ loaded or owns any listener.
    pub fn dispatch(
        mut self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<()>,
    ) -> Result<Record> {
        crate::core::deadline::check(self.deadline)?;
        self.transaction.verify_lock()?;
        self.record.outcome = if operation(&self.transaction).is_ok() {
            Outcome::Dispatched
        } else {
            Outcome::Unknown
        };
        let unknown = || {
            let mut error = AppError::new(
                "START_OUTCOME_UNKNOWN",
                "Startup dispatch may have occurred but its receipt could not be persisted; do not dispatch again.",
                5,
            );
            error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
            error.request_id = Some(self.record.request_id.clone());
            error
        };
        // The callback may have dispatched before losing its original storage authority.
        // Do not write a receipt into a replacement directory or imply that replay is safe.
        self.transaction.verify_lock().map_err(|_| unknown())?;
        save(&self.path, self.references, &self.record).map_err(|_| unknown())?;
        self.transaction.verify_lock().map_err(|_| unknown())?;
        Ok(self.record)
    }
}
fn conflict() -> AppError {
    AppError::new(
        "START_CONFLICT",
        "Startup intent, prepared run or installation no longer matches; no new dispatch occurred.",
        4,
    )
}

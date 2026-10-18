//! Durable write intent. A persisted pending record is never an invitation to retry.
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::config::read_json;
use crate::domain::{Scope, request_key};
use crate::error::{AppError, Result};
use crate::state::{Store, Transaction, atomic_json, private_dir, reject_links};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Accepted,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Record {
    pub schema_version: u8,
    pub request_id: String,
    pub operation: String,
    pub scope: Scope,
    pub payload_hash: String,
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Value>,
}

pub enum Begin<'a> {
    Existing(Record),
    Fresh(Intent<'a>),
}
pub struct Intent<'a> {
    _transaction: Transaction<'a>,
    path: PathBuf,
    record: Record,
}

fn path(store: &Store, key: &str) -> Result<PathBuf> {
    request_key(key).map_err(|_| AppError::invalid())?;
    Ok(store
        .root()
        .join("requests")
        .join(format!("{:x}.json", Sha256::digest(key.as_bytes()))))
}

/// Does not initialise state, acquire a write lock, or contact a backend.
pub fn get(store: &Store, key: &str) -> Result<Record> {
    let path = path(store, key)?;
    reject_links(&path)?;
    let record: Record = read_json(&path, 64 * 1024).map_err(|error| {
        if error.code == "FILE_NOT_FOUND" {
            AppError::new(
                "REQUEST_NOT_FOUND",
                "No durable request exists for this key.",
                4,
            )
        } else {
            error
        }
    })?;
    if record.schema_version != 1 || record.request_id != key {
        return Err(AppError::storage());
    }
    crate::domain::request_key(&record.scope.instance_id).map_err(|_| AppError::storage())?;
    crate::domain::decimal_id(&record.scope.account).map_err(|_| AppError::storage())?;
    if record.payload_hash.len() != 64
        || !record
            .payload_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || record.operation.is_empty()
        || record.operation.len() > 64
        || !record
            .operation
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(AppError::storage());
    }
    Ok(record)
}

pub fn begin<'a>(
    store: &'a Store,
    key: &str,
    operation: &str,
    scope: &Scope,
    payload: &Value,
    deadline: Instant,
) -> Result<Begin<'a>> {
    let path = path(store, key)?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(payload).map_err(|_| AppError::invalid())?)
    );
    let transaction = store.transaction(deadline)?;
    crate::request_keys::guard_message(store, key)?;
    match get(store, key) {
        Ok(mut record) => {
            // A caller may move to another instance of the same account. A key never migrates accounts or operations.
            if record.scope.account != scope.account
                || record.operation != operation
                || record.payload_hash != digest
            {
                return Err(AppError::new(
                    "REQUEST_CONFLICT",
                    "Request key already identifies another account, operation or payload.",
                    4,
                ));
            }
            // Owning the lock proves the previous writer no longer owns this intent.
            if record.outcome == Outcome::Pending {
                record.outcome = Outcome::Unknown;
                atomic_json(&path, &record)?;
            }
            Ok(Begin::Existing(record))
        }
        Err(error) if error.code == "REQUEST_NOT_FOUND" => {
            private_dir(path.parent().ok_or_else(AppError::storage)?)?;
            let record = Record {
                schema_version: 1,
                request_id: key.into(),
                operation: operation.into(),
                scope: scope.clone(),
                payload_hash: digest,
                outcome: Outcome::Pending,
                receipt: None,
            };
            // The state lock stays held through the write and final receipt commit.
            atomic_json(&path, &record)?;
            Ok(Begin::Fresh(Intent {
                _transaction: transaction,
                path,
                record,
            }))
        }
        Err(error) => Err(error),
    }
}

impl Intent<'_> {
    pub(crate) fn attachment_context(&self) -> (&Store, &Record) {
        (self._transaction.store(), &self.record)
    }
    pub fn finish(mut self, outcome: Outcome, receipt: Option<Value>) -> Result<Record> {
        if outcome == Outcome::Pending {
            return Err(AppError::invalid());
        }
        self.record.outcome = outcome;
        self.record.receipt = receipt;
        atomic_json(&self.path, &self.record)
            .map_err(|_| unknown(&self.record.request_id).with_scope(self.record.scope.clone()))?;
        Ok(self.record)
    }
}

pub fn unknown(key: &str) -> AppError {
    let mut error = AppError::new(
        "WRITE_OUTCOME_UNKNOWN",
        "The write outcome is unknown; inspect this request and do not resend automatically.",
        5,
    );
    error.outcome = Some("unknown");
    error.request_id = Some(key.into());
    error.next_action = Some(Box::new(
        serde_json::json!({"command":"request.get","args":{"request_id":key}}),
    ));
    error
}

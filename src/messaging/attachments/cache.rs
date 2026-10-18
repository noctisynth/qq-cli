//! Journaled cleanup of old, signed attachment snapshots with definite request outcomes.
use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::attachments::{MARKER, Owned, Pointer, verify_bytes};
use crate::requests::ledger::{self, Outcome};
use crate::storage::references::References;
use crate::storage::state::{Store, atomic_json, private_dir, reject_links};
use serde_json::json;

pub const RETENTION_SECONDS: u64 = 24 * 60 * 60;
pub const MAX_CACHE_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 256;
pub(crate) const MARKER_RESERVATION: u64 = 1024;

/// Caller holds the state lock through the following copy/publication.
pub(crate) fn reserve(store: &Store, incoming: u64, quota: u64, deadline: Instant) -> Result<()> {
    let root = store.root().join("attachments");
    reject_links(&root)?;
    let mut bytes = 0_u64;
    let mut entries = 0_usize;
    if root.try_exists().map_err(|_| AppError::storage())? {
        for entry in std::fs::read_dir(&root)
            .map_err(|_| AppError::storage())?
            .take(MAX_ENTRIES + 1)
        {
            if Instant::now() >= deadline {
                return Err(AppError::new(
                    "DEADLINE",
                    "Attachment quota scan exceeded its deadline.",
                    3,
                ));
            }
            let entry = entry.map_err(|_| AppError::storage())?;
            let path = entry.path();
            reject_links(&path)?;
            entries += 1;
            if entries >= MAX_ENTRIES {
                return Err(quota_error());
            }
            let metadata = entry.metadata().map_err(|_| AppError::storage())?;
            if metadata.is_file() {
                bytes = bytes.saturating_add(metadata.len());
                continue;
            }
            if !metadata.is_dir() {
                return Err(AppError::storage());
            }
            let mut count = 0;
            let mut marker = 0;
            for file in std::fs::read_dir(&path)
                .map_err(|_| AppError::storage())?
                .take(4)
            {
                let file = file.map_err(|_| AppError::storage())?;
                reject_links(&file.path())?;
                count += 1;
                if count > 3 {
                    return Err(quota_error());
                }
                let metadata = file.metadata().map_err(|_| AppError::storage())?;
                if !metadata.is_file() {
                    return Err(quota_error());
                }
                if file.file_name() == MARKER {
                    marker = metadata.len();
                } else {
                    bytes = bytes.saturating_add(metadata.len());
                }
            }
            bytes = bytes.saturating_add(marker.max(MARKER_RESERVATION));
        }
    }
    if bytes
        .saturating_add(incoming)
        .saturating_add(MARKER_RESERVATION)
        > quota
    {
        return Err(quota_error());
    }
    Ok(())
}

fn quota_error() -> AppError {
    let mut error = AppError::new(
        "CACHE_QUOTA_EXCEEDED",
        "Attachment cache capacity or bounded inventory is exhausted; existing files were preserved.",
        6,
    );
    error.next_action = Some(Box::new(json!({"command":"media.prune","args":{}})));
    error
}

pub fn prune(
    store: &Store,
    references: &References,
    instance: Option<&str>,
    now: SystemTime,
    deadline: Instant,
) -> Result<Envelope> {
    let now = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::storage())?
        .as_secs();
    if let Some(instance) = instance {
        crate::core::domain::request_key(instance).map_err(|_| AppError::invalid())?;
    }
    let _transaction = store.transaction(deadline)?;
    let root = store.root().join("attachments");
    let journals = store.root().join("attachment-removals");
    reject_links(&root)?;
    reject_links(&journals)?;
    let mut names = BTreeSet::new();
    for (path, journal) in [(&root, false), (&journals, true)] {
        if !path.try_exists().map_err(|_| AppError::storage())? {
            continue;
        }
        for entry in std::fs::read_dir(path)
            .map_err(|_| AppError::storage())?
            .take(257)
        {
            let entry = entry.map_err(|_| AppError::storage())?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(AppError::storage)?;
            let name = if journal {
                name.strip_suffix(".json").ok_or_else(AppError::storage)?
            } else {
                name
            };
            if names.len() >= 256 && !names.contains(name) {
                return Err(AppError::new(
                    "CACHE_SCAN_LIMIT",
                    "Attachment cache exceeds the bounded maintenance scan.",
                    6,
                ));
            }
            names.insert(name.to_owned());
        }
    }
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    let mut removed = 0_u64;
    let mut preserved = 0_u64;
    for name in names {
        if Instant::now() >= deadline {
            return Err(AppError::new(
                "DEADLINE",
                "Attachment cleanup exceeded its deadline; completed removals remain committed.",
                3,
            ));
        }
        if crate::core::domain::request_key(&name).is_err() {
            preserved += 1;
            continue;
        }
        let directory = root.join(&name);
        let journal = journals.join(format!("{name}.json"));
        let result = (|| {
            reject_links(&directory)?;
            reject_links(&journal)?;
            let pending = journal.try_exists().map_err(|_| AppError::storage())?;
            let marker = directory.join(MARKER);
            let pointer: Pointer =
                crate::storage::references::checkpoint(if pending { &journal } else { &marker })?;
            if pointer.schema_version != 1 {
                return Err(AppError::storage());
            }
            let owned: Owned = references.get("a1", &pointer.reference)?;
            let expected = root
                .canonicalize()
                .map_err(|_| AppError::storage())?
                .join(&name);
            if owned.directory != expected
                || instance.is_some_and(|id| id != owned.scope.instance_id)
            {
                return Ok(false);
            }
            crate::napcat::install::archive::member_path(&owned.name)
                .map_err(|_| AppError::storage())?;
            if Path::new(&owned.name).components().count() != 1
                || owned.name.eq_ignore_ascii_case(MARKER)
            {
                return Err(AppError::storage());
            }
            if now.saturating_sub(owned.created_at_seconds) < RETENTION_SECONDS {
                return Ok(false);
            }
            let record = ledger::get(store, &owned.request_id)?;
            if record.scope != owned.scope
                || owned
                    .intent_payload_hash
                    .as_ref()
                    .is_some_and(|hash| hash != &record.payload_hash)
                || !matches!(record.outcome, Outcome::Accepted | Outcome::Rejected)
            {
                return Ok(false);
            }
            if directory.try_exists().map_err(|_| AppError::storage())? {
                let entries = std::fs::read_dir(&directory)
                    .map_err(|_| AppError::storage())?
                    .take(3)
                    .collect::<std::io::Result<Vec<_>>>()
                    .map_err(|_| AppError::storage())?;
                if entries.iter().any(|entry| {
                    entry.file_name() != MARKER && entry.file_name() != owned.name.as_str()
                }) {
                    return Err(AppError::storage());
                }
                let payload = directory.join(&owned.name);
                let exists = payload.try_exists().map_err(|_| AppError::storage())?;
                if exists {
                    verify_bytes(&payload, &owned, deadline)?;
                }
                if marker.try_exists().map_err(|_| AppError::storage())? {
                    let current: Pointer = crate::storage::references::checkpoint(&marker)?;
                    if current.reference != pointer.reference || current.schema_version != 1 {
                        return Err(AppError::storage());
                    }
                } else if !pending {
                    return Err(AppError::storage());
                }
                if !pending {
                    private_dir(&journals)?;
                    atomic_json(&journal, &pointer)?;
                }
                if exists {
                    std::fs::remove_file(&payload).map_err(|_| AppError::storage())?;
                    files += 1;
                    bytes = bytes.saturating_add(owned.size);
                }
                if marker.try_exists().map_err(|_| AppError::storage())? {
                    std::fs::remove_file(&marker).map_err(|_| AppError::storage())?;
                }
                std::fs::remove_dir(&directory).map_err(|_| AppError::storage())?;
                removed += 1;
            } else if !pending {
                return Err(AppError::storage());
            }
            std::fs::remove_file(&journal).map_err(|_| AppError::storage())?;
            Ok(true)
        })();
        match result {
            Ok(true) => (),
            Ok(false) => preserved += 1,
            Err(error) if error.code == "DEADLINE" => return Err(error),
            Err(_) => preserved += 1,
        }
    }
    Ok(Envelope::success(
        json!({"cache":"attachments","removedFiles":files,"removedBytes":bytes,"removedEntries":removed,"preservedEntries":preserved,"retentionSeconds":RETENTION_SECONDS}),
    ))
}

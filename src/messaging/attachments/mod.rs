use crate::core::contracts::write::Fingerprint;
// Snapshot explicitly supplied files before a future write intent is bound to bytes.
// Retained snapshots have signed request/scope ownership; no backend action occurs here.
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::core::domain::{Scope, decimal_id, request_key};
use crate::core::error::{AppError, Result};
use crate::messaging::media::{MAX_DOWNLOAD_BYTES, Transfer, copy_from_cache};
use crate::storage::references::References;
use crate::storage::state::{Store, atomic_json, private_dir, reject_links};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;

pub use crate::core::contracts::write::Kind;
pub(crate) mod tree;

#[derive(Default)]
struct Binding<'a> {
    intent_hash: Option<&'a str>,
    persistent: bool,
}
fn transient(value: &bool) -> bool {
    !*value
}
#[must_use]
pub struct Snapshot {
    directory: tempfile::TempDir,
    path: PathBuf,
    name: String,
    kind: Kind,
    transfer: Transfer,
    source_directory: bool,
}

impl Snapshot {
    pub(crate) fn is_directory(&self) -> bool {
        self.source_directory
    }
    pub(crate) fn logical_fingerprint(&self) -> Result<serde_json::Value> {
        let mut value =
            serde_json::to_value(self.fingerprint()).map_err(|_| AppError::invalid())?;
        if self.source_directory {
            value["kind"] = serde_json::json!("directory");
        }
        Ok(value)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Stable logical content identity excludes source and random staging paths.
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint {
            kind: self.kind,
            name: self.name.clone(),
            size_bytes: self.transfer.size_bytes,
            sha256: self.transfer.sha256.clone(),
        }
    }
    /// Sign request/scope ownership before any asynchronous write. The caller must
    /// still associate the snapshot with its durable intent; never delete on unknown.
    pub fn retain(
        self,
        store: &Store,
        references: &References,
        request: &str,
        scope: &Scope,
        deadline: Instant,
    ) -> Result<Retained> {
        validate_binding(request, scope)?;
        let _transaction = store.transaction(deadline)?;
        self.retain_bound(
            store,
            references,
            request,
            scope,
            Binding::default(),
            deadline,
        )
    }
    pub(crate) fn retain_for_intent(
        self,
        intent: &crate::requests::ledger::Intent<'_>,
        references: &References,
        deadline: Instant,
    ) -> Result<Retained> {
        let (store, record) = intent.attachment_context();
        self.retain_bound(
            store,
            references,
            &record.request_id,
            &record.scope,
            Binding {
                intent_hash: Some(&record.payload_hash),
                persistent: false,
            },
            deadline,
        )
    }
    pub(crate) fn retain_persistent_for_intent(
        self,
        intent: &crate::requests::ledger::Intent<'_>,
        references: &References,
        deadline: Instant,
    ) -> Result<Retained> {
        let (store, record) = intent.attachment_context();
        self.retain_bound(
            store,
            references,
            &record.request_id,
            &record.scope,
            Binding {
                intent_hash: Some(&record.payload_hash),
                persistent: true,
            },
            deadline,
        )
    }
    fn retain_bound(
        self,
        store: &Store,
        references: &References,
        request: &str,
        scope: &Scope,
        binding: Binding<'_>,
        deadline: Instant,
    ) -> Result<Retained> {
        let directory = owned_directory(self.directory.path(), store)?;
        let owned = Owned {
            directory: directory.clone(),
            request_id: request.into(),
            scope: scope.clone(),
            name: self.name.clone(),
            kind: self.kind,
            size: self.transfer.size_bytes,
            sha256: self.transfer.sha256.clone(),
            created_at_seconds: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| ownership())?
                .as_secs(),
            intent_payload_hash: binding.intent_hash.map(str::to_owned),
            persistent: binding.persistent || self.source_directory,
            source_directory: self.source_directory,
        };
        verify_bytes(&directory.join(&owned.name), &owned, deadline)?;
        let reference = references.issue("a1", &owned)?;
        atomic_json(
            &directory.join(MARKER),
            &Pointer {
                schema_version: 1,
                reference,
            },
        )?;
        let retained = Retained {
            directory,
            path: owned.directory.join(&owned.name),
            fingerprint: self.fingerprint(),
        };
        let _ = self.directory.keep();
        Ok(retained)
    }
}

#[must_use]
pub struct Retained {
    directory: PathBuf,
    path: PathBuf,
    fingerprint: Fingerprint,
}

impl Retained {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn fingerprint(&self) -> &Fingerprint {
        &self.fingerprint
    }
}

pub(crate) const MARKER: &str = ".qq-cli-attachment.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Owned {
    #[serde(default, skip_serializing_if = "transient")]
    pub(crate) source_directory: bool,
    #[serde(default, skip_serializing_if = "transient")]
    pub(crate) persistent: bool,
    pub(crate) directory: PathBuf,
    pub(crate) request_id: String,
    pub(crate) scope: Scope,
    pub(crate) name: String,
    pub(crate) kind: Kind,
    pub(crate) size: u64,
    pub(crate) sha256: String,
    pub(crate) created_at_seconds: u64,
    #[serde(default)]
    pub(crate) intent_payload_hash: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Pointer {
    pub(crate) schema_version: u8,
    pub(crate) reference: String,
}

pub fn load(
    directory: &Path,
    store: &Store,
    references: &References,
    request: &str,
    scope: &Scope,
    deadline: Instant,
) -> Result<Retained> {
    validate_binding(request, scope)?;
    let _transaction = store.transaction(deadline)?;
    let directory = owned_directory(directory, store)?;
    let pointer: Pointer = crate::storage::references::checkpoint(&directory.join(MARKER))?;
    if pointer.schema_version != 1 {
        return Err(ownership());
    }
    let owned: Owned = references.get("a1", &pointer.reference)?;
    if owned.directory != directory || owned.scope != *scope || owned.request_id != request {
        return Err(ownership());
    }
    if let Some(hash) = &owned.intent_payload_hash {
        let intent = crate::requests::ledger::get(store, request)?;
        if intent.payload_hash != *hash || intent.scope != *scope {
            return Err(ownership());
        }
    }
    crate::napcat::install::archive::member_path(&owned.name).map_err(|_| ownership())?;
    if owned.name.eq_ignore_ascii_case(MARKER) || Path::new(&owned.name).components().count() != 1 {
        return Err(ownership());
    }
    let entries = std::fs::read_dir(&directory)
        .map_err(|_| ownership())?
        .take(if owned.source_directory { 4 } else { 3 })
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|_| ownership())?;
    if entries.len() != if owned.source_directory { 3 } else { 2 }
        || entries.iter().any(|entry| {
            entry.file_name() != MARKER
                && entry.file_name() != owned.name.as_str()
                && !(owned.source_directory && entry.file_name() == tree::MARKER)
        })
    {
        return Err(ownership());
    }
    let path = directory.join(&owned.name);
    verify_bytes(&path, &owned, deadline)?;
    Ok(Retained {
        directory,
        path,
        fingerprint: Fingerprint {
            kind: owned.kind,
            name: owned.name,
            size_bytes: owned.size,
            sha256: owned.sha256,
        },
    })
}

fn validate_binding(request: &str, scope: &Scope) -> Result<()> {
    request_key(request).map_err(|_| ownership())?;
    request_key(&scope.instance_id).map_err(|_| ownership())?;
    decimal_id(&scope.account).map_err(|_| ownership())?;
    Ok(())
}
fn owned_directory(path: &Path, store: &Store) -> Result<PathBuf> {
    reject_links(path)?;
    let path = path.canonicalize().map_err(|_| ownership())?;
    let parent = store
        .root()
        .join("attachments")
        .canonicalize()
        .map_err(|_| ownership())?;
    if path.parent() != Some(parent.as_path()) || !path.is_dir() {
        return Err(ownership());
    }
    Ok(path)
}
pub(crate) fn verify_bytes(path: &Path, owned: &Owned, deadline: Instant) -> Result<()> {
    if owned.source_directory {
        return tree::verify(path, owned, deadline);
    }
    reject_links(path)?;
    let mut file = std::fs::File::open(path).map_err(|_| ownership())?;
    let metadata = file.metadata().map_err(|_| ownership())?;
    if !metadata.is_file()
        || metadata.len() != owned.size
        || owned.size == 0
        || owned.size > MAX_DOWNLOAD_BYTES
    {
        return Err(ownership());
    }
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        if Instant::now() >= deadline {
            return Err(AppError::new(
                "DEADLINE",
                "Attachment verification exceeded its deadline.",
                3,
            ));
        }
        let count = file.read(&mut buffer).map_err(|_| ownership())?;
        if count == 0 {
            break;
        }
        total = total.saturating_add(count as u64);
        if total > owned.size {
            return Err(ownership());
        }
        hash.update(&buffer[..count]);
    }
    if total != owned.size || format!("{:x}", hash.finalize()) != owned.sha256 {
        return Err(ownership());
    }
    Ok(())
}
fn ownership() -> AppError {
    AppError::new(
        "ATTACHMENT_OWNERSHIP_INVALID",
        "Retained attachment does not match its signed ownership, request, scope or bytes.",
        4,
    )
}

pub fn prepare(source: &Path, kind: Kind, store: &Store, deadline: Instant) -> Result<Snapshot> {
    prepare_with_quota(
        source,
        kind,
        store,
        crate::messaging::attachments::cache::MAX_CACHE_BYTES,
        deadline,
    )
}

pub(crate) fn prepare_for_run(
    source: &Path,
    kind: Kind,
    store: &Store,
    instance: &crate::storage::state::Instance,
    deadline: Instant,
) -> Result<Snapshot> {
    prepare_with_prefix(
        source,
        kind,
        store,
        cache::MAX_CACHE_BYTES,
        &run_prefix(instance)?,
        deadline,
    )
}
fn run_prefix(instance: &crate::storage::state::Instance) -> Result<String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let runtime =
            crate::napcat::runtime::runtime_store::identity(&instance.id, &instance.run_id)?;
        crate::napcat::runtime::runtime_store::attachment_prefix(&runtime)
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = instance;
        Ok("prepare-".into())
    }
}
pub(crate) fn prepare_resource_for_run(
    source: &Path,
    store: &Store,
    instance: &crate::storage::state::Instance,
    deadline: Instant,
) -> Result<Snapshot> {
    reject_links(source).map_err(|_| invalid())?;
    if source.is_dir() {
        tree::prepare(source, store, instance, deadline)
    } else {
        prepare_for_run(source, Kind::File, store, instance, deadline)
    }
}
pub(crate) fn validate_resource(source: &Path, deadline: Instant) -> Result<()> {
    reject_links(source).map_err(|_| invalid())?;
    if source.is_dir() {
        tree::validate(source, deadline)
    } else {
        validate_source(source)
    }
}

fn prepare_with_quota(
    source: &Path,
    kind: Kind,
    store: &Store,
    quota: u64,
    deadline: Instant,
) -> Result<Snapshot> {
    prepare_with_prefix(source, kind, store, quota, "prepare-", deadline)
}

fn prepare_with_prefix(
    source: &Path,
    kind: Kind,
    store: &Store,
    quota: u64,
    prefix: &str,
    deadline: Instant,
) -> Result<Snapshot> {
    if Instant::now() >= deadline {
        return Err(AppError::new(
            "DEADLINE",
            "Attachment preparation exceeded its deadline.",
            3,
        ));
    }
    let (source, name, size) = source_info(source)?;
    let root = source.parent().ok_or_else(invalid)?.to_path_buf();
    let staging = store.root().join("attachments");
    let _transaction = store.transaction(deadline)?;
    crate::messaging::attachments::cache::reserve(store, size, quota, deadline)?;
    private_dir(&staging)?;
    let directory = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(&staging)
        .map_err(|_| AppError::storage())?;
    crate::storage::state::restrict_private(directory.path())?;
    let path = directory.path().join(&name);
    let transfer = copy_from_cache(&source, &[root], &path, Some(size), deadline)?;
    Ok(Snapshot {
        directory,
        path,
        name,
        kind,
        transfer,
        source_directory: false,
    })
}

pub(crate) fn validate_source(source: &Path) -> Result<()> {
    source_info(source).map(|_| ())
}
fn source_info(source: &Path) -> Result<(PathBuf, String, u64)> {
    let source = std::path::absolute(source).map_err(|_| invalid())?;
    reject_links(&source).map_err(|_| invalid())?;
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(invalid)?
        .to_owned();
    crate::napcat::install::archive::member_path(&name).map_err(|_| invalid())?;
    if name.eq_ignore_ascii_case(MARKER) {
        return Err(invalid());
    }
    let metadata = std::fs::File::open(&source)
        .map_err(|_| invalid())?
        .metadata()
        .map_err(|_| invalid())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_DOWNLOAD_BYTES {
        return Err(invalid());
    }
    Ok((source, name, metadata.len()))
}

fn invalid() -> AppError {
    AppError::new(
        "ATTACHMENT_INPUT_INVALID",
        "Attachment must be an explicit readable nonempty regular local file within the client size limit.",
        2,
    )
}

#[cfg(test)]
mod quota_tests;

pub mod cache;

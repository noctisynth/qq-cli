use std::collections::BTreeMap;
use std::fs;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use std::fs::{File, OpenOptions, TryLockError};
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use std::{thread, time::Duration};

use serde::{Deserialize, Serialize};
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
use tempfile::NamedTempFile;

use crate::core::domain::{decimal_id, request_key};
use crate::core::error::{AppError, Result};
use crate::storage::config::read_json;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub installation_id: String,
    pub endpoint: String,
    pub ws_endpoint: Option<String>,
    pub credential_key: String,
    pub account: Option<String>,
    pub run_id: String,
    pub state: String,
    pub process_id: Option<u32>,
    pub process_started: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub version: String,
    pub digest: String,
    pub directory: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Registry {
    pub schema_version: u8,
    pub installations: Vec<Installation>,
    pub instances: Vec<Instance>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub maintenance_intents: BTreeMap<String, String>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            installations: vec![],
            instances: vec![],
            maintenance_intents: BTreeMap::new(),
        }
    }
}

pub struct Store {
    root: PathBuf,
}
pub struct Transaction<'a> {
    store: &'a Store,
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    _lock: File,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    lock: durable::Lock,
    pub registry: Registry,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn read(&self) -> Result<Registry> {
        let path = self.root.join("registry.json");
        reject_links(&path)?;
        let registry: Registry = match read_json(&path, 4 * 1024 * 1024) {
            Ok(value) => value,
            Err(error) if error.code == "FILE_NOT_FOUND" => Registry::default(),
            Err(error) => return Err(error),
        };
        if registry.schema_version != 1 {
            return Err(AppError::storage());
        }
        let mut seen = std::collections::HashSet::new();
        for instance in &registry.instances {
            request_key(&instance.id).map_err(|_| AppError::storage())?;
            request_key(&instance.run_id).map_err(|_| AppError::storage())?;
            if !seen.insert(&instance.id) {
                return Err(AppError::storage());
            }
            if let Some(account) = &instance.account {
                decimal_id(account).map_err(|_| AppError::storage())?;
            }
        }
        for (id, reference) in &registry.maintenance_intents {
            request_key(id).map_err(|_| AppError::storage())?;
            if !seen.contains(&id)
                || !(reference.starts_with("v1.") || reference.starts_with("v2."))
                || reference.len() > 128
            {
                return Err(AppError::storage());
            }
        }
        Ok(registry)
    }
    pub fn transaction(&self, deadline: Instant) -> Result<Transaction<'_>> {
        private_dir(&self.root)?;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            let lock = durable::Lock::acquire(&self.root, "state.lock", deadline, "STATE_BUSY")?;
            let registry = self.read()?;
            lock.verify()?;
            Ok(Transaction {
                store: self,
                registry,
                lock,
            })
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            reject_links(&self.root.join("state.lock"))?;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(self.root.join("state.lock"))
                .map_err(|_| AppError::storage())?;
            loop {
                match file.try_lock() {
                    Ok(()) => break,
                    Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(TryLockError::WouldBlock) => {
                        return Err(AppError::new(
                            "STATE_BUSY",
                            "Local state is busy; no operation was attempted.",
                            6,
                        )
                        .safe_to_retry());
                    }
                    Err(_) => return Err(AppError::storage()),
                }
            }
            Ok(Transaction {
                registry: self.read()?,
                store: self,
                _lock: file,
            })
        }
    }
}

impl Transaction<'_> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) fn lock_identity(&self) -> Result<(u64, u64, u64, u64)> {
        self.lock.identity()
    }
    /// Revalidate the held platform lock before effects or publishing their receipt.
    pub(crate) fn verify_lock(&self) -> Result<()> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        self.lock.verify()?;
        Ok(())
    }
    pub(crate) fn store(&self) -> &Store {
        self.store
    }
    pub fn commit(self) -> Result<()> {
        self.persist()
    }
    /// Commit without releasing the lock, for operations with a subsequent journal update.
    pub fn persist(&self) -> Result<()> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            self.lock.publish_registry(&self.registry)
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            atomic_json(&self.store.root.join("registry.json"), &self.registry)
        }
    }
}

pub fn select(registry: &Registry, requested: Option<&str>) -> Result<Instance> {
    if let Some(id) = requested {
        return registry
            .instances
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or_else(|| {
                AppError::new(
                    "INSTANCE_NOT_FOUND",
                    "The selected instance is not registered.",
                    4,
                )
            });
    }
    match registry.instances.as_slice() {
        [instance] => Ok(instance.clone()),
        [] => Err(AppError::new(
            "INSTANCE_NOT_FOUND",
            "No instance is registered; use napcat start.",
            4,
        )),
        candidates => Err(AppError::new(
            "INSTANCE_AMBIGUOUS",
            "Select an instance with --instance.",
            2,
        )
        .with_details(crate::core::contracts::failure::Details::Candidates {
            candidates: candidates
                .iter()
                .take(20)
                .map(|i| crate::core::contracts::failure::Candidate {
                    instance_id: i.id.clone(),
                    state: i.state.clone(),
                    account: i.account.clone(),
                })
                .collect(),
            truncated: candidates.len() > 20,
        })),
    }
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        durable::atomic_json(path, value)
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let parent = path.parent().ok_or_else(AppError::storage)?;
        reject_links(path)?;
        let mut temporary = NamedTempFile::new_in(parent).map_err(|_| AppError::storage())?;
        serde_json::to_writer(&mut temporary, value).map_err(|_| AppError::storage())?;
        temporary
            .write_all(b"\n")
            .map_err(|_| AppError::storage())?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| AppError::storage())?;
        temporary.persist(path).map_err(|_| AppError::storage())?;
        Ok(())
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "state/durable.rs"]
pub(crate) mod durable;

pub fn reject_links(path: &Path) -> Result<()> {
    let absolute = std::path::absolute(path).map_err(|_| AppError::storage())?;
    for parent in absolute.ancestors() {
        if let Ok(metadata) = fs::symlink_metadata(parent) {
            #[cfg(windows)]
            let linked = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let linked = metadata.file_type().is_symlink();
            if linked {
                return Err(AppError::new(
                    "UNSAFE_PATH",
                    "Symbolic links or reparse points are not allowed in private state.",
                    6,
                ));
            }
        }
    }
    Ok(())
}

pub fn private_dir(path: &Path) -> Result<()> {
    reject_links(path)?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        durable::private_dir(path)
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        if path.exists() {
            return Ok(());
        }
        fs::create_dir_all(path).map_err(|_| AppError::storage())?;
        restrict_private(path)
    }
}

/// Restrict a newly owned file or directory; never apply this to a user parent directory.
pub fn restrict_private(path: &Path) -> Result<()> {
    reject_links(path)?;
    let directory = path.metadata().map_err(|_| AppError::storage())?.is_dir();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if directory { 0o700 } else { 0o600 }),
        )
        .map_err(|_| AppError::storage())?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let system = std::env::var_os("SystemRoot").ok_or_else(AppError::storage)?;
        let bin = PathBuf::from(system).join("System32");
        let user = Command::new(bin.join("whoami.exe"))
            .args(["/user", "/fo", "csv", "/nh"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|_| AppError::storage())?;
        if !user.status.success() {
            return Err(AppError::storage());
        }
        let text = String::from_utf8(user.stdout).map_err(|_| AppError::storage())?;
        let sid = text
            .split('"')
            .find(|part| {
                part.starts_with("S-1-")
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_digit() || matches!(b, b'S' | b'-'))
            })
            .ok_or_else(AppError::storage)?;
        let grant = format!("*{sid}:{}", if directory { "(OI)(CI)F" } else { "F" });
        let result = Command::new(bin.join("icacls.exe"))
            .arg(path)
            .args(["/inheritance:r", "/grant:r", &grant])
            .creation_flags(0x08000000)
            .output()
            .map_err(|_| AppError::storage())?;
        if !result.status.success() {
            return Err(AppError::storage());
        }
    }
    reject_links(path)
}

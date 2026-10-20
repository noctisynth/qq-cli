use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use serde::{Serialize, de::DeserializeOwned};
use sha2::Sha256;

use crate::core::error::{AppError, Result};
use crate::storage::config::read_json;
use crate::storage::credentials::Credentials;
use crate::storage::state::{Store, private_dir, reject_links};

/// Compact authenticated handles. Callers store locator metadata, never message bodies.
pub struct References {
    directory: PathBuf,
    key: [u8; 32],
}

impl References {
    /// Load an established signing key without creating state or provisioning credentials.
    pub fn existing(store: &Store, credentials: &impl Credentials) -> Result<Self> {
        let encoded = credentials.get("reference-signing-key")?;
        let key: [u8; 32] = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| AppError::storage())?
            .try_into()
            .map_err(|_| AppError::storage())?;
        Ok(Self::with_key(store.root().join("references"), key))
    }
    pub fn open(store: &Store, credentials: &impl Credentials, deadline: Instant) -> Result<Self> {
        let _transaction = store.transaction(deadline)?;
        let key = match credentials.get("reference-signing-key") {
            Ok(encoded) => URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| AppError::storage())?,
            Err(error) if error.code == "CREDENTIAL_NOT_FOUND" => {
                if signing_material_exists(store)? {
                    return Err(AppError::new(
                        "SIGNING_KEY_REQUIRED",
                        "Existing signed state requires its original private signing key; no replacement was created.",
                        4,
                    ));
                }
                let mut key = [0_u8; 32];
                getrandom::fill(&mut key).map_err(|_| AppError::storage())?;
                credentials.set("reference-signing-key", &URL_SAFE_NO_PAD.encode(key))?;
                key.to_vec()
            }
            Err(error) => return Err(error),
        };
        let key: [u8; 32] = key.try_into().map_err(|_| AppError::storage())?;
        Ok(Self::with_key(store.root().join("references"), key))
    }
    pub fn with_key(directory: PathBuf, key: [u8; 32]) -> Self {
        Self { directory, key }
    }
    pub fn issue<T: Serialize>(&self, kind: &str, value: &T) -> Result<String> {
        validate_kind(kind)?;
        private_dir(&self.directory)?;
        let bytes = serde_json::to_vec(value).map_err(|_| AppError::storage())?;
        if bytes.len() > reference_limit(kind) {
            return Err(AppError::storage());
        }
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| AppError::storage())?;
        let nonce = URL_SAFE_NO_PAD.encode(nonce);
        let prefix = format!("{kind}.{nonce}");
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| AppError::storage())?;
        mac.update(prefix.as_bytes());
        mac.update(&bytes);
        let tag = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        let path = self.directory.join(format!("{prefix}.json"));
        let mut file = std::fs::File::create_new(path).map_err(|_| AppError::storage())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| AppError::storage())?;
        Ok(format!("{prefix}.{tag}"))
    }
    pub fn get<T: DeserializeOwned>(&self, kind: &str, token: &str) -> Result<T> {
        validate_kind(kind)?;
        let bad = || {
            AppError::new(
                "REFERENCE_INVALID",
                "Reference is invalid, missing, modified or from another state directory.",
                2,
            )
        };
        if token.len() > 128 {
            return Err(bad());
        }
        let parts = token.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts[0] != kind {
            return Err(bad());
        }
        if URL_SAFE_NO_PAD
            .decode(parts[1])
            .ok()
            .is_none_or(|n| n.len() != 16)
        {
            return Err(bad());
        }
        let tag = URL_SAFE_NO_PAD.decode(parts[2]).map_err(|_| bad())?;
        let prefix = format!("{}.{}", parts[0], parts[1]);
        let path = self.directory.join(format!("{prefix}.json"));
        reject_links(&path)?;
        // Read a JSON byte array indirectly only after a bounded file read.
        let bytes = {
            use std::io::Read;
            let file = std::fs::File::open(&path).map_err(|_| bad())?;
            let mut bytes = Vec::new();
            file.take(reference_limit(kind) as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| bad())?;
            if bytes.len() > reference_limit(kind) {
                return Err(bad());
            }
            bytes
        };
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| bad())?;
        mac.update(prefix.as_bytes());
        mac.update(&bytes);
        mac.verify_slice(&tag).map_err(|_| bad())?;
        serde_json::from_slice(&bytes).map_err(|_| bad())
    }
}

fn signing_material_exists(store: &Store) -> Result<bool> {
    if !store.read()?.maintenance_intents.is_empty() {
        return Ok(true);
    }
    for name in [
        "references",
        "run-jobs",
        "instance-creations",
        "instance-stops",
        "instance-removals",
        "run-transitions",
        "run-origins",
        "version-maintenance",
        "automatic-start",
        "retentions",
        "start-runs",
        "start-requests",
        "runtimes",
        "runtime-preparations",
        "installations",
        "installation-operations",
        "compatibility-evidence",
        "attachments",
        "attachment-removals",
    ] {
        let path = store.root().join(name);
        reject_links(&path)?;
        match std::fs::read_dir(path) {
            Ok(mut entries) => {
                if entries
                    .next()
                    .transpose()
                    .map_err(|_| AppError::storage())?
                    .is_some()
                {
                    return Ok(true);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(AppError::storage()),
        }
    }
    Ok(false)
}

fn validate_kind(kind: &str) -> Result<()> {
    if matches!(
        kind,
        "m1" | "w1"
            | "c1"
            | "f1"
            | "h1"
            | "e1"
            | "i1"
            | "u1"
            | "a1"
            | "r1"
            | "s1"
            | "t1"
            | "j1"
            | "n1"
            | "d1"
            | "p1"
            | "x1"
            | "g1"
            | "v1"
            | "k1"
    ) {
        Ok(())
    } else {
        Err(AppError::storage())
    }
}

fn reference_limit(kind: &str) -> usize {
    if kind == "i1" {
        4 * 1024 * 1024
    } else {
        64 * 1024
    }
}

/// For private checkpoints that are not exposed as public handles.
pub fn checkpoint<T: DeserializeOwned>(path: &std::path::Path) -> Result<T> {
    reject_links(path)?;
    read_json(path, 64 * 1024)
}

#[cfg(test)]
mod maintenance_tests;
#[cfg(test)]
mod signing_tests;

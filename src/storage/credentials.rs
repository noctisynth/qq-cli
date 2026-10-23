use crate::core::error::{AppError, Result};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod macos;

pub trait Credentials {
    fn get(&self, key: &str) -> Result<String>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

pub struct SystemCredentials {
    service: String,
    #[cfg(windows)]
    legacy_service: Option<String>,
}

impl SystemCredentials {
    pub fn new(state_root: &std::path::Path) -> Result<Self> {
        use sha2::{Digest, Sha256};
        crate::storage::state::reject_links(state_root)?;
        let absolute = std::path::absolute(state_root).map_err(|_| AppError::storage())?;
        let mut ancestor = absolute.as_path();
        let mut missing = Vec::new();
        while !ancestor.try_exists().map_err(|_| AppError::storage())? {
            missing.push(
                ancestor
                    .file_name()
                    .ok_or_else(AppError::storage)?
                    .to_owned(),
            );
            ancestor = ancestor.parent().ok_or_else(AppError::storage)?;
        }
        let mut canonical = ancestor.canonicalize().map_err(|_| AppError::storage())?;
        for part in missing.into_iter().rev() {
            canonical.push(part);
        }
        let identity = canonical.to_str().ok_or_else(AppError::storage)?;
        #[cfg(windows)]
        let identity = identity.to_lowercase();
        let digest = Sha256::digest(identity.as_bytes());
        // Packaged Windows hosts may redirect a newly created leaf even though
        // its existing parent still resolves to the original directory. Earlier
        // releases provisioned keys before creating that leaf.
        #[cfg(windows)]
        let legacy_service =
            absolute
                .parent()
                .zip(absolute.file_name())
                .and_then(|(parent, leaf)| {
                    let mut original = parent.canonicalize().ok()?;
                    original.push(leaf);
                    let original = original.to_str()?;
                    #[cfg(windows)]
                    let original = original.to_lowercase();
                    let legacy = format!("qq-cli-{:x}", Sha256::digest(original.as_bytes()));
                    (legacy != format!("qq-cli-{:x}", digest)).then_some(legacy)
                });
        Ok(Self {
            service: format!("qq-cli-{:x}", digest),
            #[cfg(windows)]
            legacy_service,
        })
    }
}

#[cfg(windows)]
impl SystemCredentials {
    /// Select the original authority for the entire credential set. Never merge
    /// two independent signers or copy/replace their secrets.
    fn selected_service(&self) -> Result<&str> {
        let Some(legacy) = self.legacy_service.as_deref() else {
            return Ok(&self.service);
        };
        let signing_key = |service: &str| -> Result<Option<String>> {
            match keyring::Entry::new(service, "reference-signing-key")
                .and_then(|e| e.get_password())
            {
                Ok(key) => Ok(Some(key)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(_) => Err(AppError::new(
                    "CREDENTIAL_UNAVAILABLE",
                    "Private signing authority is unavailable.",
                    4,
                )),
            }
        };
        match (signing_key(&self.service)?, signing_key(legacy)?) {
            (Some(current), Some(original)) if current != original => Err(AppError::new(
                "CREDENTIAL_NAMESPACE_CONFLICT",
                "Resolved and original state locations have different signing authorities; neither was modified.",
                4,
            )),
            (None, Some(_)) => Ok(legacy),
            _ => Ok(&self.service),
        }
    }
}

#[cfg(windows)]
impl Credentials for SystemCredentials {
    fn get(&self, key: &str) -> Result<String> {
        let _access = crate::native::credential_lock::acquire()?;
        keyring::Entry::new(self.selected_service()?, key)
            .and_then(|entry| entry.get_password())
            .map_err(|error| match error {
                keyring::Error::NoEntry => AppError::new(
                    "CREDENTIAL_NOT_FOUND",
                    "Private credentials have not been provisioned.",
                    4,
                ),
                _ => AppError::new(
                    "CREDENTIAL_UNAVAILABLE",
                    "Private backend credentials are unavailable.",
                    4,
                ),
            })
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        let _access = crate::native::credential_lock::acquire()?;
        keyring::Entry::new(self.selected_service()?, key)
            .and_then(|entry| entry.set_password(value))
            .map_err(|_| {
                AppError::new(
                    "CREDENTIAL_STORAGE",
                    "Credentials could not be protected by the operating system.",
                    6,
                )
            })
    }
    fn delete(&self, key: &str) -> Result<()> {
        let _access = crate::native::credential_lock::acquire()?;
        let entry =
            keyring::Entry::new(self.selected_service()?, key).map_err(|_| AppError::storage())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(AppError::storage()),
        }
    }
}

#[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
impl Credentials for SystemCredentials {
    fn get(&self, _key: &str) -> Result<String> {
        let _ = &self.service;
        Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Protected credential storage requires Windows or Apple Silicon macOS.",
            4,
        ))
    }
    fn set(&self, _key: &str, _value: &str) -> Result<()> {
        self.get("").map(|_| ())
    }
    fn delete(&self, _key: &str) -> Result<()> {
        self.get("").map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::SystemCredentials;
    #[cfg(windows)]
    #[test]
    fn redirected_namespace_reuses_original_authority_and_rejects_independent_signers() {
        use super::Credentials;
        let temp = tempfile::tempdir().unwrap();
        let primary = SystemCredentials::new(&temp.path().join("primary")).unwrap();
        let original = SystemCredentials::new(&temp.path().join("original")).unwrap();
        struct Cleanup<'a>(&'a SystemCredentials);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-api");
                let _ = self.0.delete("reference-signing-key");
            }
        }
        let _cleanup_primary = Cleanup(&primary);
        let _cleanup_original = Cleanup(&original);
        original
            .set("reference-signing-key", "synthetic-original-authority")
            .unwrap();
        original
            .set("synthetic-api", "synthetic-original-api")
            .unwrap();
        let redirected = SystemCredentials {
            service: primary.service.clone(),
            legacy_service: Some(original.service.clone()),
        };
        assert_eq!(
            redirected.get("synthetic-api").unwrap(),
            "synthetic-original-api"
        );
        redirected
            .set("synthetic-api", "synthetic-updated-api")
            .unwrap();
        assert_eq!(
            original.get("synthetic-api").unwrap(),
            "synthetic-updated-api"
        );
        assert_eq!(
            primary.get("synthetic-api").unwrap_err().code,
            "CREDENTIAL_NOT_FOUND"
        );
        primary
            .set("reference-signing-key", "synthetic-other-authority")
            .unwrap();
        assert_eq!(
            redirected.get("synthetic-api").unwrap_err().code,
            "CREDENTIAL_NAMESPACE_CONFLICT"
        );
        assert_eq!(
            redirected
                .set("synthetic-api", "must-not-write")
                .unwrap_err()
                .code,
            "CREDENTIAL_NAMESPACE_CONFLICT"
        );
        assert_eq!(
            redirected.delete("synthetic-api").unwrap_err().code,
            "CREDENTIAL_NAMESPACE_CONFLICT"
        );
        assert_eq!(
            original.get("synthetic-api").unwrap(),
            "synthetic-updated-api"
        );
    }
    #[test]
    fn vault_namespaces_use_the_resolved_state_location_not_relative_spelling() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let missing = SystemCredentials::new(&first).unwrap().service;
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&second).unwrap();
        assert_eq!(missing, SystemCredentials::new(&first).unwrap().service);
        assert_eq!(
            missing,
            SystemCredentials::new(&first.join(".")).unwrap().service
        );
        assert_ne!(missing, SystemCredentials::new(&second).unwrap().service);
        assert_eq!(
            SystemCredentials::new(std::path::Path::new("."))
                .unwrap()
                .service,
            SystemCredentials::new(&std::env::current_dir().unwrap())
                .unwrap()
                .service
        );
        #[cfg(windows)]
        assert_eq!(
            missing,
            SystemCredentials::new(std::path::Path::new(
                &first.to_str().unwrap().to_uppercase()
            ))
            .unwrap()
            .service
        );
    }
}

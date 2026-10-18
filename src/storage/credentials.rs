use crate::core::error::{AppError, Result};

pub trait Credentials {
    fn get(&self, key: &str) -> Result<String>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

pub struct SystemCredentials {
    service: String,
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
        Ok(Self {
            service: format!("qq-cli-{:x}", digest),
        })
    }
}

#[cfg(windows)]
impl Credentials for SystemCredentials {
    fn get(&self, key: &str) -> Result<String> {
        let _access = crate::native::credential_lock::acquire()?;
        keyring::Entry::new(&self.service, key)
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
        keyring::Entry::new(&self.service, key)
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
        let entry = keyring::Entry::new(&self.service, key).map_err(|_| AppError::storage())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(AppError::storage()),
        }
    }
}

#[cfg(not(windows))]
impl Credentials for SystemCredentials {
    fn get(&self, _key: &str) -> Result<String> {
        let _ = &self.service;
        Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Protected credential storage is currently supported on Windows only.",
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

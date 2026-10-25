//! Default Keychain access is noninteractive; explicit authorization is isolated.
use super::{Credentials, SystemCredentials};
use crate::core::error::{AppError, Result};
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::passwords::{self, PasswordOptions};
use std::sync::Mutex;

// SecKeychain's interaction setting is process-wide. Serialize changes made by this adapter.
static ACCESS: Mutex<()> = Mutex::new(());
pub(super) mod authorization;
// Security.framework's public errSecItemNotFound value.
const ITEM_NOT_FOUND: i32 = -25300;

fn access<T>(operation: impl FnOnce() -> Result<T>) -> Result<T> {
    let _access = ACCESS.lock().map_err(|_| unavailable())?;
    let allowed = SecKeychain::user_interaction_allowed().map_err(|_| unavailable())?;
    // The library guard restores true on Drop; do not create it if UI was already disabled.
    let _interaction = if allowed {
        Some(SecKeychain::disable_user_interaction().map_err(|_| unavailable())?)
    } else {
        None
    };
    operation()
}
fn options(service: &str, key: &str) -> Result<PasswordOptions> {
    // Empty fields are wildcards in some Keychain APIs. Reject before any vault access.
    if service.is_empty() || key.is_empty() || key.len() > 256 || key.chars().any(char::is_control)
    {
        return Err(AppError::invalid());
    }
    let mut options = PasswordOptions::new_generic_password(service, key);
    options.set_access_synchronized(Some(false));
    Ok(options)
}
impl Credentials for SystemCredentials {
    fn get(&self, key: &str) -> Result<String> {
        let options = options(&self.service, key)?;
        access(|| {
            let bytes = passwords::generic_password(options).map_err(|error| {
                if error.code() == ITEM_NOT_FOUND {
                    AppError::new(
                        "CREDENTIAL_NOT_FOUND",
                        "Private credentials have not been provisioned.",
                        4,
                    )
                } else {
                    unavailable()
                }
            })?;
            String::from_utf8(bytes).map_err(|_| unavailable())
        })
    }
    fn set(&self, key: &str, value: &str) -> Result<()> {
        let options = options(&self.service, key)?;
        access(|| {
            passwords::set_generic_password_options(value.as_bytes(), options).map_err(|_| {
                AppError::new(
                    "CREDENTIAL_STORAGE",
                    "Credentials could not be protected by the operating system.",
                    6,
                )
            })
        })
    }
    fn delete(&self, key: &str) -> Result<()> {
        let options = options(&self.service, key)?;
        access(
            || match passwords::delete_generic_password_options(options) {
                Ok(()) => Ok(()),
                Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
                Err(_) => Err(unavailable()),
            },
        )
    }
}
fn unavailable() -> AppError {
    AppError::new(
        "CREDENTIAL_UNAVAILABLE",
        "Private Keychain credentials are unavailable without user interaction. Unlock the login keychain; for an intended instance, explicitly run napcat start --authorize-keychain and approve the exact CLI in system dialogs using Always Allow. Retain the previous executable and do not delete or replace credentials.",
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_or_control_keys_are_rejected_before_vault_access() {
        for key in ["", "bad\nkey", "bad\0key"] {
            assert!(options("synthetic-service", key).is_err());
        }
        assert!(options("", "synthetic-key").is_err());
        assert!(options("synthetic-service", &"a".repeat(257)).is_err());
    }

    #[test]
    #[ignore = "explicit native Keychain test; creates and removes unique synthetic credentials"]
    fn native_keychain_roundtrip_isolated_namespaces_and_cleanup() {
        let interaction = SecKeychain::user_interaction_allowed().unwrap();
        assert!(access(|| Err::<(), _>(AppError::invalid())).is_err());
        assert_eq!(
            SecKeychain::user_interaction_allowed().unwrap(),
            interaction
        );
        let temp = tempfile::tempdir().unwrap();
        let first = SystemCredentials::new(&temp.path().join("first")).unwrap();
        let second = SystemCredentials::new(&temp.path().join("second")).unwrap();
        struct Cleanup<'a>(&'a SystemCredentials);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.delete("synthetic-key");
            }
        }
        let _cleanup = Cleanup(&first);
        assert_eq!(
            first.get("synthetic-key").unwrap_err().code,
            "CREDENTIAL_NOT_FOUND"
        );
        first.set("synthetic-key", "synthetic-value-one").unwrap();
        assert_eq!(first.get("synthetic-key").unwrap(), "synthetic-value-one");
        assert_eq!(
            second.get("synthetic-key").unwrap_err().code,
            "CREDENTIAL_NOT_FOUND"
        );
        first.set("synthetic-key", "synthetic-value-two").unwrap();
        assert_eq!(first.get("synthetic-key").unwrap(), "synthetic-value-two");
        let worker = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "storage::credentials::macos::tests::native_keychain_worker",
                "--ignored",
                "--test-threads=1",
            ])
            .env("QQ_CLI_SYNTHETIC_KEYCHAIN_SERVICE", &first.service)
            .output()
            .unwrap();
        assert!(
            worker.status.success(),
            "same executable Keychain recovery failed"
        );
        assert_eq!(
            first.get("synthetic-key").unwrap_err().code,
            "CREDENTIAL_NOT_FOUND"
        );
        first.delete("synthetic-key").unwrap();
        first.delete("synthetic-key").unwrap();
        assert_eq!(
            first.get("synthetic-key").unwrap_err().code,
            "CREDENTIAL_NOT_FOUND"
        );
        assert_eq!(
            SecKeychain::user_interaction_allowed().unwrap(),
            interaction
        );
    }

    #[test]
    #[ignore = "child entry for the explicit synthetic native Keychain test"]
    fn native_keychain_worker() {
        let Ok(service) = std::env::var("QQ_CLI_SYNTHETIC_KEYCHAIN_SERVICE") else {
            return;
        };
        let vault = SystemCredentials { service };
        assert_eq!(vault.get("synthetic-key").unwrap(), "synthetic-value-two");
        vault.delete("synthetic-key").unwrap();
    }
}

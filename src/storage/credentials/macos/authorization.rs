//! Explicit OS authorization in a bounded read-only SDK worker. No secret IPC.
use super::{ITEM_NOT_FOUND, SystemCredentials, options, unavailable};
use crate::core::error::{AppError, Result};
use crate::native::macos_signature::{FAILURE, TIMEOUT, check_worker, watchdogs};
use crate::storage::credentials::Credentials;
use security_framework::os::macos::keychain::SecKeychain;
use security_framework::passwords;
use std::ffi::{OsStr, OsString};
use std::time::{Duration, Instant};

const FLAG: &str = "--internal-macos-keychain-authorization-v1";
const AUTHORIZED: u8 = 96;
const SIGNER: &str = "reference-signing-key";
const MAX_AUTHORIZATION_MS: u64 = 1_800_000;

impl SystemCredentials {
    /// Only an explicit human authorization flow may call this. The worker
    /// returns a status, never credentials, and cannot create/delete items.
    pub(crate) fn authorize_existing(
        &self,
        keys: &[String],
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<()> {
        validate(&self.service, keys)?;
        let parameters = std::iter::once(OsStr::new(&self.service))
            .chain(keys.iter().map(OsStr::new))
            .collect::<Vec<_>>();
        let code = check_worker(
            FLAG,
            &parameters,
            deadline,
            Duration::from_millis(MAX_AUTHORIZATION_MS),
            &guard,
        ).map_err(|mut error| {
            if error.code == "SIGNATURE_WORKER_CLEANUP_UNKNOWN" {
                AppError::new(
                    "CREDENTIAL_AUTHORIZATION_UNKNOWN",
                    "Keychain authorization worker exit is unconfirmed. Inspect the system dialog; no credential result was accepted.",
                    4,
                )
            } else if error.code == "QQ_SIGNATURE_UNVERIFIED" {
                unavailable()
            } else {
                if error.code == "DEADLINE" {
                    error.message = "Keychain authorization exceeded the command timeout; no startup was dispatched. Dismiss any remaining system prompt before retrying with a longer --timeout.";
                }
                error
            }
        })?;
        if code != AUTHORIZED {
            return Err(unavailable());
        }
        // Allow Once in the disposable worker may not grant later CLI access.
        // Recheck through the normal no-UI adapter before claiming success.
        for (index, key) in keys.iter().enumerate() {
            guard()?;
            crate::core::deadline::check(deadline)?;
            match self.get(key) {
                Ok(_) => (),
                Err(error) if index > 0 && error.code == "CREDENTIAL_NOT_FOUND" => (),
                Err(error) => return Err(error),
            }
        }
        crate::core::deadline::check(deadline)?;
        guard()
    }
}

fn hash(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn validate(service: &str, keys: &[String]) -> Result<()> {
    if !service.strip_prefix("qq-cli-").is_some_and(hash)
        || keys.first().map(String::as_str) != Some(SIGNER)
    {
        return Err(AppError::invalid());
    }
    match keys {
        [_] => Ok(()),
        [_, bundle, api] => {
            let suffix = bundle
                .strip_prefix("runtime-bundle-")
                .ok_or_else(AppError::invalid)?;
            if !hash(suffix) || api != &format!("runtime-api-{suffix}") {
                return Err(AppError::invalid());
            }
            Ok(())
        }
        _ => Err(AppError::invalid()),
    }
}

fn request(args: &[OsString]) -> Result<(&str, Vec<String>, Duration)> {
    let (service, rest) = args.split_first().ok_or_else(AppError::invalid)?;
    let (budget, keys) = rest.split_last().ok_or_else(AppError::invalid)?;
    let milliseconds = budget
        .to_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| (1..=MAX_AUTHORIZATION_MS).contains(v))
        .ok_or_else(AppError::invalid)?;
    let service = service.to_str().ok_or_else(AppError::invalid)?;
    let keys = keys
        .iter()
        .map(|s| s.to_str().map(str::to_owned).ok_or_else(AppError::invalid))
        .collect::<Result<Vec<_>>>()?;
    validate(service, &keys)?;
    Ok((service, keys, Duration::from_millis(milliseconds)))
}

/// Only explicitly selected fixed items; no vault enumeration or secret output.
pub fn worker_entry() -> Option<u8> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.first().map(OsString::as_os_str) != Some(OsStr::new(FLAG)) {
        return None;
    }
    let Ok((service, keys, duration)) = request(&args[1..]) else {
        return Some(FAILURE);
    };
    let deadline = Instant::now() + duration;
    if watchdogs(duration).is_err() || !matches!(SecKeychain::user_interaction_allowed(), Ok(true))
    {
        return Some(FAILURE);
    }
    for (index, key) in keys.iter().enumerate() {
        if Instant::now() >= deadline {
            return Some(TIMEOUT);
        }
        let Ok(options) = options(service, key) else {
            return Some(FAILURE);
        };
        match passwords::generic_password(options) {
            Ok(_) => (),
            Err(error) if index > 0 && error.code() == ITEM_NOT_FOUND => (),
            Err(_) => return Some(FAILURE),
        }
        if Instant::now() >= deadline {
            return Some(TIMEOUT);
        }
    }
    Some(AUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expiry_and_lost_guard_refuse_before_any_authorization_worker_or_vault_access() {
        let temp = tempfile::tempdir().unwrap();
        let vault = SystemCredentials::new(&temp.path().canonicalize().unwrap()).unwrap();
        let keys = vec![SIGNER.into()];
        assert_eq!(
            vault
                .authorize_existing(&keys, Instant::now() - Duration::from_secs(1), || panic!(
                    "expired guard"
                ))
                .unwrap_err()
                .code,
            "DEADLINE"
        );
        assert_eq!(
            vault
                .authorize_existing(&keys, Instant::now() + Duration::from_secs(1), || Err(
                    AppError::invalid()
                ))
                .unwrap_err()
                .code,
            "INVALID_ARGUMENTS"
        );
    }
    #[test]
    fn authorization_scope_cannot_expand_to_other_services_items_or_runs() {
        let service = format!("qq-cli-{}", "a".repeat(64));
        let keys = vec![
            SIGNER.into(),
            format!("runtime-bundle-{}", "b".repeat(64)),
            format!("runtime-api-{}", "b".repeat(64)),
        ];
        assert!(validate(&service, &keys).is_ok());
        assert!(validate(&service, &keys[..1]).is_ok());
        for bad_service in ["", "other-service", "qq-cli-short"] {
            assert!(validate(bad_service, &keys).is_err());
        }
        for invalid in [
            vec![],
            vec!["".into()],
            vec![SIGNER.into(), keys[1].clone()],
            vec![
                SIGNER.into(),
                keys[1].clone(),
                format!("runtime-api-{}", "c".repeat(64)),
            ],
            vec![SIGNER.into(), "arbitrary-item".into(), keys[2].clone()],
        ] {
            assert!(validate(&service, &invalid).is_err());
        }
        for budget in ["0", "1800001", "invalid"] {
            assert!(request(&[service.clone().into(), SIGNER.into(), budget.into()]).is_err());
        }
    }

    #[test]
    fn explicit_authorization_accepts_the_cli_budget_beyond_five_minutes() {
        let service = format!("qq-cli-{}", "a".repeat(64));
        for milliseconds in [1, 300_001, 900_000, MAX_AUTHORIZATION_MS] {
            let (_, keys, budget) = request(&[
                service.clone().into(),
                SIGNER.into(),
                milliseconds.to_string().into(),
            ])
            .unwrap();
            assert_eq!(keys, [SIGNER]);
            assert_eq!(budget, Duration::from_millis(milliseconds));
        }
    }
}

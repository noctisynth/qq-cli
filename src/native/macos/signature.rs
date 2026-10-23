//! Static original-QQ signatures through safe Security Framework wrappers.
//! No execution, resigning, Keychain item queries or network revocation fetches.
use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;
use core_foundation::url::CFURL;
use security_framework::os::macos::code_signing::{Flags, SecRequirement, SecStaticCode};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Instant;
#[path = "signature/worker.rs"]
mod worker;
pub use worker::{verify_isolated, worker_entry};

const DEVELOPER_ID: &str = r#"anchor apple generic and identifier "com.tencent.qq" and certificate 1[field.1.2.840.113635.100.6.2.6] exists and certificate leaf[field.1.2.840.113635.100.6.1.13] exists and certificate leaf[subject.OU] = "FN2V63AD2J""#;
const APP_STORE: &str = r#"anchor apple generic and identifier "com.tencent.qq" and certificate leaf[field.1.2.840.113635.100.6.1.9] exists"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    TencentDeveloperId,
    AppleAppStore,
}

/// A current observation, never an installation/launch/ownership permit.
/// The OS call is synchronous and cannot be cancelled by this wrapper; reject
/// a late result, but do not claim a hard wall-clock bound while inside it.
pub fn verify_original(root: &Path, deadline: Instant) -> Result<Origin> {
    crate::core::deadline::check(deadline)?;
    reject_links(root)?;
    if !root.is_absolute() || root.extension().is_none_or(|ext| ext != "app") {
        return Err(invalid());
    }
    let canonical = root.canonicalize().map_err(|_| invalid())?;
    if canonical.as_os_str() != root.as_os_str() {
        return Err(invalid());
    }
    let before = std::fs::symlink_metadata(root).map_err(|_| invalid())?;
    if !before.is_dir() {
        return Err(invalid());
    }
    let url = CFURL::from_path(root, true).ok_or_else(invalid)?;
    let code = SecStaticCode::from_path(&url, Flags::NONE).map_err(|_| invalid())?;
    let flags = Flags::STRICT_VALIDATE
        | Flags::CHECK_NESTED_CODE
        | Flags::CHECK_ALL_ARCHITECTURES
        | Flags::NO_NETWORK_ACCESS;
    let developer: SecRequirement = DEVELOPER_ID.parse().map_err(|_| invalid())?;
    let store: SecRequirement = APP_STORE.parse().map_err(|_| invalid())?;
    let origin = if code.check_validity(flags, &developer).is_ok() {
        Origin::TencentDeveloperId
    } else {
        crate::core::deadline::check(deadline)?;
        code.check_validity(flags, &store).map_err(|_| invalid())?;
        Origin::AppleAppStore
    };
    crate::core::deadline::check(deadline)?;
    reject_links(root)?;
    let after = std::fs::symlink_metadata(root).map_err(|_| invalid())?;
    if (
        before.dev(),
        before.ino(),
        before.mode(),
        before.uid(),
        before.gid(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.mode(),
        after.uid(),
        after.gid(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return Err(invalid());
    }
    Ok(origin)
}

fn invalid() -> AppError {
    AppError::new(
        "QQ_SIGNATURE_UNVERIFIED",
        "Original QQ code signature, sealed resources or publisher could not be verified.",
        3,
    )
}

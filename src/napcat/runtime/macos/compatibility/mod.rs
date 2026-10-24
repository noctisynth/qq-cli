//! Successful startup evidence is separate from static preparation or API reachability.
mod key;
mod local;
mod observation;
use crate::core::error::{AppError, Result};
use crate::storage::{references::References, state::Store};
pub use key::Key;
pub use observation::record;

pub fn has_evidence(store: &Store) -> Result<bool> {
    local::present(store)
}
pub fn supported(store: &Store, refs: &References, key: &Key) -> Result<bool> {
    local::supported(store, refs, key)
}
pub fn assessment(
    store: &Store,
    refs: Option<&References>,
) -> Result<crate::core::contracts::diagnostic::Compatibility> {
    use crate::core::contracts::diagnostic::{Compatibility, Verdict};
    let key = Key::current()?;
    let fingerprint = key.fingerprint()?;
    let supported = match refs {
        Some(refs) => local::supported(store, refs, &key)?,
        None => false,
    };
    Ok(Compatibility {
        state: if supported {
            Verdict::Supported
        } else {
            Verdict::Unknown
        },
        reason: if supported {
            "EXACT_TUPLE_EVIDENCE"
        } else {
            "NO_EXACT_FRAMEWORK_EVIDENCE"
        }
        .into(),
        key_fingerprint: Some(fingerprint.clone()),
        evidence: if supported {
            vec![format!("LOCAL_MACOS_FRAMEWORK_STARTUP:{fingerprint}")]
        } else {
            vec![]
        },
    })
}

fn unverified() -> AppError {
    AppError::new(
        "STARTUP_EVIDENCE_UNVERIFIED",
        "Original macOS launch inputs and online service could not be verified; no compatibility evidence was recorded.",
        4,
    )
}

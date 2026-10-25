//! Resolve the running image, independently of a mutable installation launcher.
use super::Pinned;
use crate::core::error::{AppError, Result};
use std::path::PathBuf;
use std::time::Instant;

/// macOS current_exe can retain the invocation symlink. Resolving that link
/// later could select a different version after an installer switches it.
/// The kernel's original process observation supplies the actual image path.
/// This is not loaded-code attestation; installed version files must be retained.
pub(crate) fn executable(deadline: Instant) -> Result<PathBuf> {
    let current = Pinned::observe(std::process::id(), deadline)?;
    let image = current.identity().image().to_owned();
    crate::storage::state::reject_links(&image)?;
    if !std::fs::symlink_metadata(&image)
        .map_err(|_| unavailable())?
        .is_file()
    {
        return Err(unavailable());
    }
    current.verify(deadline)?;
    Ok(image)
}
fn unavailable() -> AppError {
    AppError::new(
        "WORKER_PROGRAM_UNVERIFIABLE",
        "The running executable image could not be resolved; no alternate worker program was selected.",
        4,
    )
}

#[cfg(test)]
mod tests;

//! Read-only target and source verification before any maintenance/stop intent.
use super::*;
use crate::napcat::{
    install::{installation, macos::source::image::acquire},
    runtime::macos::{compatibility, release},
};

pub(super) fn verify(
    context: &start::Context<'_, impl Credentials>,
    current: &Instance,
    original: &Installation,
    target: &Installation,
    trial: bool,
    deadline: Instant,
) -> Result<()> {
    for installed in [original, target] {
        let release = release::lookup(&installed.version)?;
        if installed.version != release.version || installed.digest != release.archive.sha256 {
            return Err(version_intent::conflict());
        }
        installation::verify(context.store, installed, context.references, deadline)?;
    }
    if original.id == target.id {
        return Ok(());
    }
    let supported = compatibility::supported(
        context.store,
        context.references,
        &compatibility::Key::for_version(&target.version)?,
    )?;
    if !supported && !trial {
        return Err(AppError::new(
            "COMPATIBILITY_UNKNOWN",
            "The exact macOS target combination needs positive startup evidence or an explicit update trial; no run was stopped.",
            4,
        ));
    }
    acquire::acquire(context.store, None, deadline)?;
    if matches!(current.state.as_str(), "ready" | "login_required") {
        start::observe(context, current, original, deadline)?;
    }
    Ok(())
}

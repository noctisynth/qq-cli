//! Read-only launch inputs. Successful inspection is not a process launch permit.
use std::collections::HashSet;
use std::time::Instant;

use serde_json::json;

use crate::catalog::Release;
use crate::compatibility::{self, Key, Verdict, Witness};
use crate::doctor::Probe;
use crate::error::{AppError, Result};
use crate::qq::Application;
use crate::state::{Installation, Registry, Store};

/// Contains private paths; never expose this as CLI output or serialize it.
/// The caller must retain the lifecycle lock and recheck loaded files before dispatch.
pub struct Inputs {
    installation: Installation,
    application: Application,
    compatibility: Key,
}
impl Inputs {
    pub fn installation(&self) -> &Installation {
        &self.installation
    }
    pub fn application(&self) -> &Application {
        &self.application
    }
    pub fn compatibility(&self) -> &Key {
        &self.compatibility
    }
}

pub fn inspect(
    store: &Store,
    registry: &Registry,
    release: &Release,
    witnesses: &[Witness],
    probe: &impl Probe,
    deadline: Instant,
) -> Result<Inputs> {
    crate::deadline::check(deadline)?;
    release.require_authorization()?;
    let packages = registry
        .installations
        .iter()
        .filter(|item| {
            item.version.strip_prefix('v').unwrap_or(&item.version)
                == release.version.strip_prefix('v').unwrap_or(release.version)
        })
        .collect::<Vec<_>>();
    if packages.is_empty() {
        return Err(AppError::new(
            "INSTALLATION_NOT_FOUND",
            "The fixed release has no registered installation; install it before launching.",
            4,
        ));
    }
    if packages.len() != 1 {
        return Err(AppError::new("INSTALLATION_AMBIGUOUS", "The fixed release has multiple registered installations; no launch input was selected.", 4)
            .with_details(json!({"installationIds":packages.iter().map(|item| &item.id).collect::<Vec<_>>()})));
    }
    let installation = packages[0];
    if installation.digest != release.archive.sha256
        || registry
            .installations
            .iter()
            .filter(|item| item.id == installation.id)
            .count()
            != 1
    {
        return Err(AppError::new(
            "INSTALLATION_MODIFIED",
            "Registered artifact identity does not match the fixed release.",
            4,
        ));
    }
    crate::deadline::check(deadline)?;
    probe.artifact(store, installation, deadline)?;
    crate::deadline::check(deadline)?;
    let paths = probe.qq_paths()?;
    if paths.len() > 16 {
        return Err(AppError::new(
            "QQ_DISCOVERY_LIMIT",
            "QQ installation discovery exceeded its bounded candidate set; no launch input was selected.",
            4,
        ));
    }
    let mut seen = HashSet::new();
    let mut applications = Vec::new();
    for path in paths {
        crate::deadline::check(deadline)?;
        if !path.is_absolute() {
            return Err(AppError::new(
                "QQ_METADATA_INVALID",
                "QQ discovery returned an unverifiable installation location.",
                4,
            ));
        }
        // Same source may be reported by multiple registry views. This is metadata
        // deduplication only, never discovery or adoption of running QQ processes.
        if seen.insert(path.to_string_lossy().to_lowercase()) {
            applications.push(probe.inspect_qq(&path, deadline)?);
        }
    }
    crate::deadline::check(deadline)?;
    if applications.is_empty() {
        return Err(AppError::new(
            "QQ_NOT_FOUND",
            "No verifiable local QQ installation was discovered.",
            4,
        ));
    }
    if applications.len() != 1 {
        return Err(AppError::new("QQ_INSTALLATION_AMBIGUOUS", "Multiple QQ installations were discovered; no launch input was selected.", 4)
            .with_details(json!({"qqInstallationIds":applications.iter().map(|item| &item.id).collect::<Vec<_>>()})));
    }
    let application = applications.pop().ok_or_else(AppError::storage)?;
    let key = Key::from_installation(probe.host(), release, &application).ok_or_else(|| {
        AppError::new(
            "COMPATIBILITY_UNKNOWN",
            "The fixed release does not provide all required compatibility inputs.",
            4,
        )
    })?;
    let assessment = compatibility::evaluate(&key, witnesses);
    if assessment.state != Verdict::Supported {
        return Err(AppError::new(
            if assessment.state == Verdict::Unsupported { "COMPATIBILITY_UNSUPPORTED" } else { "COMPATIBILITY_UNKNOWN" },
            "Exact Framework startup compatibility has not been established for these launch inputs.", 4)
            .with_details(json!({"reason":assessment.reason,"keyFingerprint":key.fingerprint(),"evidence":assessment.evidence})));
    }
    crate::deadline::check(deadline)?;
    Ok(Inputs {
        installation: installation.clone(),
        application,
        compatibility: key,
    })
}

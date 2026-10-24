//! Read-only managed-app diagnostics. Never mount, prepare, launch or create keys.
use super::*;
use crate::core::contracts::diagnostic::{Check, Compatibility, Detail, PreparationState, Verdict};
use crate::napcat::{
    doctor::PlatformReport,
    install::{catalog::Release, macos::source},
    runtime::macos::{compatibility, deploy},
};
use crate::storage::state::Registry;

pub(super) fn inspect(
    store: &Store,
    registry: &Registry,
    instance: Option<&Instance>,
    release: &Release,
    deadline: Instant,
) -> Result<PlatformReport> {
    crate::core::deadline::check(deadline)?;
    let supported_version = release.version == "v4.18.41";
    let (state, code) = if !supported_version {
        (PreparationState::Unavailable, "VERSION_UNSUPPORTED")
    } else {
        match deployment(store, registry, instance, deadline) {
            Ok(value) => value,
            Err(error) => (PreparationState::Unavailable, error.code),
        }
    };
    let available = !matches!(&state, PreparationState::Unavailable);
    let compatibility = if supported_version {
        match assess(store) {
            Ok(value) => value,
            Err(error) => Compatibility {
                state: Verdict::Unknown,
                reason: error.code.into(),
                key_fingerprint: None,
                evidence: vec![],
            },
        }
    } else {
        Compatibility {
            state: Verdict::Unsupported,
            reason: "VERSION_UNSUPPORTED".into(),
            key_fingerprint: None,
            evidence: vec![],
        }
    };
    let can_start = available && compatibility.state == Verdict::Supported;
    crate::core::deadline::check(deadline)?;
    Ok(PlatformReport {
        checks: vec![
            Check::Detail(Detail::ManagedQq {
                state,
                qq_version: source::QQ_VERSION.into(),
                qq_build: source::QQ_BUILD.into(),
                source_sha256: source::DMG_SHA256.into(),
                code: code.into(),
            }),
            Check::Detail(Detail::Compatibility {
                result: compatibility,
            }),
        ],
        can_start,
    })
}

fn assess(store: &Store) -> Result<Compatibility> {
    if !compatibility::has_evidence(store)? {
        return compatibility::assessment(store, None);
    }
    let credentials = SystemCredentials::new(store.root())?;
    let refs = References::existing(store, &credentials)?;
    compatibility::assessment(store, Some(&refs))
}

fn deployment(
    store: &Store,
    registry: &Registry,
    instance: Option<&Instance>,
    deadline: Instant,
) -> Result<(PreparationState, &'static str)> {
    let Some(instance) = instance else {
        return Ok((PreparationState::Required, "QQ_PREPARATION_REQUIRED"));
    };
    if !deploy::has_record(store, instance)? {
        return Ok((PreparationState::Required, "QQ_PREPARATION_REQUIRED"));
    }
    let installation = registry
        .installations
        .iter()
        .find(|v| v.id == instance.installation_id)
        .ok_or_else(crate::core::error::AppError::storage)?;
    let credentials = SystemCredentials::new(store.root())?;
    let refs = References::existing(store, &credentials)?;
    crate::napcat::install::installation::verify(store, installation, &refs, deadline)?;
    match deploy::completed_target(store, &refs, instance, deadline)? {
        Some(_) => Ok((PreparationState::Prepared, "QQ_DEPLOYMENT_VERIFIED")),
        None => Ok((PreparationState::Required, "QQ_PREPARATION_REQUIRED")),
    }
}

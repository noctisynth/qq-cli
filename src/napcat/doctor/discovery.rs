//! System-installation discovery and exact-tuple assessment; no preparation effects.
use super::*;
use crate::napcat::compatibility::Key;
use std::collections::HashSet;

pub(super) fn inspect(
    host: Host,
    release: &Release,
    witnesses: &[Witness],
    probe: &impl Probe,
    deadline: Instant,
) -> Result<PlatformReport> {
    let mut checks = Vec::new();
    let mut applications = Vec::new();
    let mut candidate_codes = Vec::new();
    let mut source_limit = false;
    crate::core::deadline::check(deadline)?;
    match probe.qq_paths() {
        Ok(paths) => {
            source_limit = paths.len() > 16;
            let mut seen = HashSet::new();
            let mut ids = HashSet::new();
            for path in paths.into_iter().take(16) {
                crate::core::deadline::check(deadline)?;
                let key = std::path::absolute(&path)
                    .map_err(|_| AppError::storage())?
                    .to_string_lossy()
                    .to_lowercase();
                if !seen.insert(key) {
                    continue;
                }
                match probe.inspect_qq(&path, deadline) {
                    Ok(application) if ids.insert(application.id.clone()) => {
                        applications.push(application)
                    }
                    Ok(_) => (),
                    Err(error) => candidate_codes.push(error.code),
                }
            }
        }
        Err(error) => candidate_codes.push(error.code),
    }
    let qq_ready = applications.len() == 1 && candidate_codes.is_empty() && !source_limit;
    checks.push(Check::Detail(Detail::Qq {
        subject: Subject::DiscoveredInstallation,
        source: Source::InstallationMetadata,
        state: if qq_ready {
            DiscoveryState::Ok
        } else if applications.is_empty() && candidate_codes.is_empty() {
            DiscoveryState::Missing
        } else {
            DiscoveryState::Unknown
        },
        candidates: applications.clone(),
        errors: candidate_codes.into_iter().map(str::to_owned).collect(),
        truncated: source_limit,
    }));
    let assessment = if qq_ready {
        Key::from_installation(host.clone(), release, &applications[0]).map(|key| {
            let assessment = compatibility::evaluate(&key, witnesses);
            let supported = assessment.state == Verdict::Supported;
            (
                Compatibility {
                    state: assessment.state,
                    reason: assessment.reason.into(),
                    key_fingerprint: Some(key.fingerprint()),
                    evidence: assessment.evidence,
                },
                supported,
            )
        })
    } else {
        None
    };
    let (compatibility, supported) = assessment.unwrap_or((
        Compatibility {
            state: Verdict::Unknown,
            reason: "COMPATIBILITY_INPUTS_UNVERIFIED".into(),
            key_fingerprint: None,
            evidence: vec![],
        },
        false,
    ));
    checks.push(Check::Detail(Detail::Compatibility {
        result: compatibility,
    }));
    Ok(PlatformReport {
        checks,
        can_start: supported,
    })
}

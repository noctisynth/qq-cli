//! Read-only diagnostics keep startup evidence, artifact integrity and live API probes separate.
mod discovery;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::core::contracts::{diagnostic::*, scalar::LocalId};

use crate::core::domain::Scope;
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::napcat::compatibility::{self, Verdict, Witness};
use crate::napcat::install::catalog::{self, Release};
use crate::napcat::qq::Application;
use crate::native::platform::Host;
use crate::storage::config::Config;
use crate::storage::state::{Installation, Instance, Registry, Store, select};
use crate::transport::client::SessionInfo;

pub trait Probe {
    fn host(&self) -> Host;
    fn qq_paths(&self) -> Result<Vec<PathBuf>>;
    fn inspect_qq(&self, path: &Path, deadline: Instant) -> Result<Application>;
    /// A managed platform may prepare its own fixed source instead of using a
    /// system installation. This observation must remain read-only.
    fn managed_platform(
        &self,
        _store: &Store,
        _registry: &Registry,
        _instance: Option<&Instance>,
        _release: &Release,
        _deadline: Instant,
    ) -> Result<Option<PlatformReport>> {
        Ok(None)
    }
    fn artifact(&self, store: &Store, installation: &Installation, deadline: Instant)
    -> Result<()>;
    fn api(
        &self,
        store: &Store,
        config: &Config,
        instance: &Instance,
        version: &str,
        deadline: Instant,
    ) -> Result<SessionInfo>;
}

pub struct PlatformReport {
    pub checks: Vec<Check>,
    pub can_start: bool,
}

pub fn execute(
    store: &Store,
    config: &Config,
    registry: &Registry,
    instance: Option<&str>,
    version: Option<&str>,
    probe: &impl Probe,
) -> Result<Envelope> {
    execute_until(
        store,
        config,
        registry,
        instance,
        version,
        probe,
        Instant::now() + Duration::from_secs(30),
    )
}

pub fn execute_until(
    store: &Store,
    config: &Config,
    registry: &Registry,
    instance: Option<&str>,
    version: Option<&str>,
    probe: &impl Probe,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let selected = if registry.instances.is_empty() && instance.is_none() {
        None
    } else {
        Some(select(registry, instance)?)
    };
    let installed = selected.as_ref().and_then(|instance| {
        registry
            .installations
            .iter()
            .find(|item| item.id == instance.installation_id)
    });
    let default_version = if probe.host().os == "macos" {
        "v4.18.41"
    } else {
        "v4.18.38"
    };
    let version = version
        .or_else(|| installed.map(|item| item.version.as_str()))
        .unwrap_or(default_version);
    let release = catalog::lookup(version)?;
    let witnesses = if probe.host().os == "macos" {
        Vec::new()
    } else {
        compatibility::available_witnesses(store)?
    };
    diagnose_until(
        store,
        config,
        registry,
        Request {
            instance: selected.as_ref(),
            release: &release,
            witnesses: &witnesses,
        },
        probe,
        deadline,
    )
}

pub fn diagnose(
    store: &Store,
    config: &Config,
    registry: &Registry,
    instance: Option<&Instance>,
    release: &Release,
    witnesses: &[Witness],
    probe: &impl Probe,
) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(30);
    diagnose_until(
        store,
        config,
        registry,
        Request {
            instance,
            release,
            witnesses,
        },
        probe,
        deadline,
    )
}

/// Fixed evidence selected for a read-only diagnostic pass.
pub struct Request<'a> {
    pub instance: Option<&'a Instance>,
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
}

pub fn diagnose_until(
    store: &Store,
    config: &Config,
    registry: &Registry,
    request: Request<'_>,
    probe: &impl Probe,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let Request {
        instance,
        release,
        witnesses,
    } = request;
    let host = probe.host();
    let mut checks = Vec::new();
    let packages = registry
        .installations
        .iter()
        .filter(|item| {
            item.version.trim_start_matches('v') == release.version.trim_start_matches('v')
                && item.digest == release.archive.sha256
        })
        .collect::<Vec<_>>();
    let package = if let Some(instance) = instance {
        packages
            .iter()
            .find(|item| item.id == instance.installation_id)
            .copied()
            .or_else(|| {
                if packages.len() == 1 {
                    packages.first().copied()
                } else {
                    None
                }
            })
    } else if packages.len() == 1 {
        packages.first().copied()
    } else {
        None
    };
    let integrity = if let Some(package) = package {
        crate::core::deadline::check(deadline)?;
        match probe.artifact(store, package, deadline) {
            Ok(()) => {
                checks.push(Check::Detail(Detail::Artifact {
                    state: ArtifactState::Ok,
                    installation_id: LocalId::new(&package.id)?,
                }));
                true
            }
            Err(error) => {
                checks.push(check(CheckName::Artifact, CheckState::Blocked, error.code));
                false
            }
        }
    } else {
        checks.push(check(
            CheckName::Artifact,
            if packages.is_empty() {
                CheckState::Missing
            } else {
                CheckState::Unknown
            },
            if packages.is_empty() {
                "INSTALLATION_NOT_FOUND"
            } else {
                "INSTALLATION_AMBIGUOUS"
            },
        ));
        false
    };
    let authorized = release
        .authorization_evidence
        .is_some_and(|value| !value.is_empty());
    checks.push(check(
        CheckName::Authorization,
        if authorized {
            CheckState::Recorded
        } else {
            CheckState::Unknown
        },
        if authorized {
            "LICENSE_ANNOTATION_RECORDED"
        } else {
            "LICENSE_REVIEW_PENDING"
        },
    ));
    let platform = match probe.managed_platform(store, registry, instance, release, deadline)? {
        Some(report) => report,
        None => discovery::inspect(host.clone(), release, witnesses, probe, deadline)?,
    };
    let supported = platform.can_start;
    checks.extend(platform.checks);
    let mut scope = None;
    if let Some(instance) = instance {
        if let Some(installation) = registry
            .installations
            .iter()
            .find(|item| item.id == instance.installation_id)
        {
            crate::core::deadline::check(deadline)?;
            match probe.api(store, config, instance, &installation.version, deadline) {
                Ok(info) => {
                    scope = Some(Scope {
                        instance_id: instance.id.clone(),
                        account: info.account,
                    });
                    checks.push(Check::Detail(Detail::Api {
                        state: if info.online {
                            ApiState::Ready
                        } else {
                            ApiState::Offline
                        },
                        online: info.online,
                        version: info.version,
                    }));
                }
                Err(error) => {
                    checks.push(check(CheckName::Api, CheckState::Unavailable, error.code))
                }
            }
        } else {
            checks.push(check(
                CheckName::Api,
                CheckState::Unavailable,
                "INSTALLATION_NOT_FOUND",
            ));
        }
    } else {
        checks.push(check(
            CheckName::Api,
            CheckState::NotStarted,
            "INSTANCE_NOT_FOUND",
        ));
    }
    crate::core::deadline::check(deadline)?;
    let mut envelope = Envelope::typed(Diagnostic {
        phase: DiagnosticPhase::Diagnostic,
        host,
        release: crate::core::contracts::diagnostic::Release {
            version: release.version.into(),
            commit: release.commit.into(),
        },
        checks,
        can_start: integrity && supported,
    });
    if let Some(scope) = scope {
        envelope = envelope.scoped(scope);
    }
    Ok(envelope)
}

fn check(name: CheckName, state: CheckState, code: &str) -> Check {
    Check::Coded(CodedCheck {
        name,
        state,
        code: code.into(),
    })
}

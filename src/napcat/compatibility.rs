//! Exact-tuple evidence. File presence and a successful API probe are not startup evidence.
pub mod local;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::napcat::install::catalog::Release;
use crate::napcat::qq::Application;
use crate::native::platform::Host;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Key {
    pub host: Host,
    pub mode: String,
    pub napcat_version: String,
    pub commit: String,
    pub archive_sha256: String,
    pub launcher_sha256: String,
    pub loader_sha256: String,
    pub qq_version: String,
    pub qq_installation_id: String,
    pub qq_build: String,
    pub qq_architecture: String,
    pub qq_fingerprint: String,
}

pub use crate::core::contracts::diagnostic::Verdict;

pub struct Witness {
    pub key: Key,
    pub verdict: Verdict,
    pub reference: String,
    pub kind: EvidenceKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    StaticInspection,
    ShellSmoke,
    FrameworkStartup,
    /// Explicit per-invocation permission; never a successful compatibility witness.
    AuthorizedStartupTrial,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub state: Verdict,
    pub reason: &'static str,
    pub evidence: Vec<String>,
}

impl Key {
    pub fn from_installation(host: Host, release: &Release, qq: &Application) -> Option<Self> {
        let required = |name| {
            release
                .archive
                .required
                .iter()
                .find(|(path, _)| *path == name)
                .map(|(_, digest)| (*digest).to_owned())
        };
        Some(Self {
            host,
            mode: "framework_native".into(),
            napcat_version: release.version.into(),
            commit: release.commit.into(),
            archive_sha256: release.archive.sha256.into(),
            launcher_sha256: required("napimain.exe")?,
            loader_sha256: required("napiloader.dll")?,
            qq_version: qq.version.clone(),
            qq_installation_id: qq.id.clone(),
            qq_build: qq.build.clone(),
            qq_architecture: qq.architecture.clone(),
            qq_fingerprint: qq.fingerprint.clone(),
        })
    }
    pub fn fingerprint(&self) -> String {
        // Stable field order; includes only software/environment evidence and an opaque local installation locator.
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).unwrap_or_default())
        )
    }
}

pub fn evaluate(key: &Key, witnesses: &[Witness]) -> Assessment {
    let result = |state, reason, evidence| Assessment {
        state,
        reason,
        evidence,
    };
    if key.host.os.is_empty() || key.host.os == "unknown" {
        return result(Verdict::Unknown, "OS_UNVERIFIED", vec![]);
    }
    if key.host.os != "windows" || key.mode != "framework_native" {
        return result(
            Verdict::Unsupported,
            "CURRENT_PLATFORM_OR_MODE_UNSUPPORTED",
            vec![],
        );
    }
    if [
        &key.host.native_arch,
        &key.host.process_arch,
        &key.qq_architecture,
    ]
    .iter()
    .any(|value| value.is_empty() || value.as_str() == "unknown")
    {
        return result(Verdict::Unknown, "ARCHITECTURE_UNVERIFIED", vec![]);
    }
    if key.host.os != "windows"
        || key.host.process_arch != "x86_64"
        || key.host.native_arch != "x86_64"
        || key.qq_architecture != "x86_64"
        || key.mode != "framework_native"
    {
        return result(
            Verdict::Unsupported,
            "CURRENT_PLATFORM_OR_MODE_UNSUPPORTED",
            vec![],
        );
    }
    let hash =
        |value: &str| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if key.host.os_build.is_none_or(|build| build == 0)
        || key.qq_build.is_empty()
        || key.qq_version.is_empty()
        || key.qq_installation_id.is_empty()
        || key.commit.len() != 40
        || !key.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || [
            &key.archive_sha256,
            &key.launcher_sha256,
            &key.loader_sha256,
            &key.qq_fingerprint,
        ]
        .iter()
        .any(|value| !hash(value))
    {
        return result(Verdict::Unknown, "INCOMPLETE_COMPATIBILITY_KEY", vec![]);
    }
    let matching = witnesses
        .iter()
        .filter(|witness| {
            witness.key == *key
                && witness.kind == EvidenceKind::FrameworkStartup
                && !witness.reference.is_empty()
                && witness.verdict != Verdict::Unknown
        })
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return result(Verdict::Unknown, "NO_EXACT_FRAMEWORK_EVIDENCE", vec![]);
    }
    let state = matching[0].verdict.clone();
    if matching.iter().any(|witness| witness.verdict != state) {
        return result(Verdict::Unknown, "EVIDENCE_CONFLICT", vec![]);
    }
    let mut evidence = matching
        .iter()
        .map(|witness| witness.reference.clone())
        .collect::<Vec<_>>();
    evidence.sort();
    evidence.dedup();
    evidence.truncate(20);
    result(state, "EXACT_TUPLE_EVIDENCE", evidence)
}

/// No complete, reproducible Framework tuple has been recorded by this project yet.
pub fn reviewed_witnesses() -> Vec<Witness> {
    Vec::new()
}

pub fn available_witnesses(
    store: &crate::storage::state::Store,
) -> crate::core::error::Result<Vec<Witness>> {
    let mut witnesses = reviewed_witnesses();
    let directory = store.root().join("compatibility-evidence");
    crate::storage::state::reject_links(&directory)?;
    if directory
        .try_exists()
        .map_err(|_| crate::core::error::AppError::storage())?
    {
        let credentials = crate::storage::credentials::SystemCredentials::new(store.root())?;
        let refs = crate::storage::references::References::existing(store, &credentials)?;
        witnesses.extend(local::load(store, &refs)?);
    }
    Ok(witnesses)
}

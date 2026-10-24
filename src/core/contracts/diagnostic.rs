//! Diagnostic wire types also used by host discovery and compatibility probes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Host {
    pub os: String,
    pub os_build: Option<u32>,
    pub native_arch: String,
    pub process_arch: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Application {
    pub id: String,
    pub version: String,
    pub build: String,
    pub architecture: String,
    pub fingerprint: String,
    pub executable_sha256: String,
    pub major_sha256: String,
    pub entry_sha256: String,
    pub metadata_sha256: String,
    #[serde(skip)]
    pub executable: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPhase {
    Diagnostic,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub version: String,
    pub commit: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Diagnostic {
    pub phase: DiagnosticPhase,
    pub host: Host,
    pub release: Release,
    pub checks: Vec<Check>,
    pub can_start: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckName {
    Artifact,
    Authorization,
    Api,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Blocked,
    Missing,
    Unknown,
    Recorded,
    Unavailable,
    NotStarted,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CodedCheck {
    pub name: CheckName,
    pub state: CheckState,
    pub code: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Check {
    Coded(CodedCheck),
    Detail(Detail),
}
#[derive(Serialize, JsonSchema)]
#[serde(tag = "name", rename_all = "snake_case", deny_unknown_fields)]
pub enum Detail {
    ManagedQq {
        state: PreparationState,
        #[serde(rename = "qqVersion")]
        qq_version: String,
        #[serde(rename = "qqBuild")]
        qq_build: String,
        #[serde(rename = "sourceSha256")]
        source_sha256: String,
        code: String,
    },
    Artifact {
        state: ArtifactState,
        #[serde(rename = "installationId")]
        installation_id: super::scalar::LocalId,
    },
    Qq {
        subject: Subject,
        source: Source,
        state: DiscoveryState,
        candidates: Vec<Application>,
        errors: Vec<String>,
        truncated: bool,
    },
    Compatibility {
        result: Compatibility,
    },
    Api {
        state: ApiState,
        online: bool,
        version: String,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreparationState {
    Required,
    Prepared,
    Unavailable,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    Ok,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    DiscoveredInstallation,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    InstallationMetadata,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryState {
    Ok,
    Missing,
    Unknown,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApiState {
    Ready,
    Offline,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Compatibility {
    pub state: Verdict,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub key_fingerprint: Option<String>,
    pub evidence: Vec<String>,
}

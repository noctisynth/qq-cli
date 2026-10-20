//! Native run capabilities. Journal, creation, service binding and prelogin retention are private implementations.
mod control;
mod journal;
mod launch;
mod prelogin;
mod service;
use crate::core::error::{AppError, Result};
use crate::napcat::runtime::start_intent::Target;
use crate::native::job::{Job, JobId};
use crate::native::process::{Identity, Pinned};
pub use control::{ControlRun, reopen_control};
pub use journal::has_record;
pub(crate) use journal::observed_target;
pub(crate) use journal::owned_job_id;
pub use launch::{launch_for_service_once, launch_once, reopen_root};
pub use prelogin::retain_prelogin;
use serde::{Deserialize, Serialize};
pub use service::{bind_service, discover_registered, discover_service, reopen_service};
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Pending,
    Spawned,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    target: Target,
    job: JobId,
    outcome: Outcome,
    root: Option<Identity>,
    #[serde(default)]
    service: Option<Service>,
    #[serde(default)]
    discovery: Option<Discovery>,
    #[serde(default)]
    prelogin: Option<Service>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Discovery {
    image: PathBuf,
    endpoint: String,
    #[serde(default)]
    control_endpoint: Option<String>,
}
pub struct ServiceExpectation<'a> {
    pub image: &'a Path,
    pub control_endpoint: &'a str,
    pub ports: crate::napcat::runtime::runtime_config::Ports,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Service {
    owner: Identity,
    endpoint: String,
}
/// Verified service capability. Contains private process paths and must not be serialized.
pub struct ServiceRun {
    job: Job,
    owner: Pinned,
}
impl ServiceRun {
    pub fn job(&self) -> &Job {
        &self.job
    }
    pub fn owner(&self) -> &Pinned {
        &self.owner
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
/// Owns the observed root and Job. Dropping does not terminate either process or Job members.
pub struct Running {
    job: Job,
    root: Pinned,
}
impl Running {
    pub fn job(&self) -> &Job {
        &self.job
    }
    pub fn root(&self) -> &Pinned {
        &self.root
    }
}
fn conflict() -> AppError {
    AppError::new(
        "RUN_JOB_CONFLICT",
        "The run's durable dispatch or original process does not match; no process was created.",
        4,
    )
}
fn process_image_path(path: &Path) -> Result<PathBuf> {
    let text = path.to_str().ok_or_else(conflict)?;
    if text.chars().any(char::is_control) {
        return Err(conflict());
    }
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        let bytes = rest.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || bytes[2] != b'\\'
        {
            return Err(conflict());
        }
        return Ok(rest.into());
    }
    let bytes = text.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        return Err(conflict());
    }
    Ok(path.to_owned())
}
fn unknown() -> AppError {
    let mut e = AppError::new(
        "START_OUTCOME_UNKNOWN",
        "Native dispatch may have occurred; do not repeat it automatically.",
        5,
    );
    e.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
    e
}

#[cfg(test)]
mod tests;

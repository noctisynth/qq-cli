//! Retain one query-only job handle in a verified member's handle table, without executing code there.
use super::{Job, JobId, OwnedHandle, QUERY, pin, unavailable};
use crate::core::error::{AppError, Result};
use crate::native::process::{Identity, Pinned};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use windows::Win32::Foundation::{DUPLICATE_HANDLE_OPTIONS, DuplicateHandle, HANDLE};
use windows::Win32::System::JobObjects::IsProcessInJob;
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE,
};

/// Private receipt, no Debug or public-output adapter. The target owns the remote
/// handle until it closes it or exits. This type must never close that numeric value locally.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Retained {
    schema_version: u8,
    job: JobId,
    target: Identity,
    remote_handle: u64,
}
impl Retained {
    pub(crate) fn validate_for(&self, job: &JobId, target: &Identity) -> Result<()> {
        if self.schema_version != 1
            || &self.job != job
            || &self.target != target
            || self.remote_handle == 0
            || self.remote_handle == u64::MAX
        {
            return Err(AppError::storage());
        }
        Ok(())
    }
    pub fn target(&self) -> &Identity {
        &self.target
    }
}
impl Job {
    /// Caller MUST persist an at-most-once retention intent before calling.
    /// This is a handle-table operation, not DLL/thread injection or a general process adoption API.
    pub fn retain_in(&self, process: &Pinned, deadline: Instant) -> Result<Retained> {
        crate::core::deadline::check(deadline)?;
        self.check_limits(deadline)?;
        if !self.contains(process, deadline)? {
            return Err(not_member());
        }
        // SAFETY: fixed minimal access, noninheritable result; PID comes from a live pinned identity,
        // but is NOT trusted until the newly opened object's complete identity is compared below.
        let target = unsafe {
            OpenProcess(
                PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                process.identity().process_id(),
            )
        }
        .map_err(|_| unavailable())?;
        let target = pin(OwnedHandle(target), deadline)?;
        if target.identity() != process.identity() {
            return Err(not_member());
        }
        let raw = HANDLE(target.native_handle().ptr());
        member(self, raw, deadline)?;
        process.verify(deadline)?;
        target.verify(deadline)?;
        let mut remote = HANDLE::default();
        crate::core::deadline::check(deadline)?;
        // SAFETY: borrowed live source job and target process objects, valid writable out pointer.
        // The duplicate belongs to TARGET, has QUERY only, is noninheritable and is never locally owned.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                self.handle.0,
                raw,
                &mut remote,
                QUERY,
                false,
                DUPLICATE_HANDLE_OPTIONS(0),
            )
        }
        .map_err(|_| unknown())?;
        if remote.is_invalid() {
            return Err(unknown());
        }
        // Verification failure after duplication is uncertain, never permission to duplicate again.
        member(self, raw, deadline).map_err(|_| unknown())?;
        target.verify(deadline).map_err(|_| unknown())?;
        process.verify(deadline).map_err(|_| unknown())?;
        Ok(Retained {
            schema_version: 1,
            job: self.id().clone(),
            target: target.identity().clone(),
            remote_handle: remote.0 as usize as u64,
        })
    }
}
fn member(job: &Job, process: HANDLE, deadline: Instant) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    let mut found = false.into();
    // SAFETY: both borrowed handles stay live; writable BOOL is correctly typed and initialized.
    unsafe { IsProcessInJob(process, Some(job.handle.0), &mut found) }
        .map_err(|_| unavailable())?;
    if found.as_bool() {
        Ok(())
    } else {
        Err(not_member())
    }
}
fn not_member() -> AppError {
    AppError::new(
        "JOB_MEMBER_MISMATCH",
        "The pinned process is not a verified member of this run; no handle was retained.",
        4,
    )
}
fn unknown() -> AppError {
    let mut error = AppError::new(
        "RETENTION_OUTCOME_UNKNOWN",
        "A process handle-table operation may have occurred; do not repeat it automatically.",
        5,
    );
    error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
    error
}

#[cfg(test)]
mod tests;

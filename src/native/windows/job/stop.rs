//! Explicit maintenance only. Query handles and inherited handles never gain termination rights.
use super::{Job, JobId, OwnedHandle, QUERY, unavailable};
use crate::core::error::{AppError, Result};
use std::time::Instant;
use windows::{
    Win32::System::JobObjects::{OpenJobObjectW, TerminateJobObject},
    core::PCWSTR,
};
#[cfg(test)]
mod tests;

pub(crate) struct StopJob {
    job: Job,
}
impl StopJob {
    /// Caller must first validate the original signed run and hold the lifecycle lock.
    pub(crate) fn reopen(id: &JobId, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let name = id.wide()?;
        // SAFETY: validated NUL-terminated private job name; unique noninheritable handle.
        // No assign/set-attributes rights are requested. DACL enforces the same owner boundary.
        let handle = unsafe { OpenJobObjectW(QUERY | 0x0008, false, PCWSTR(name.as_ptr())) }
            .map_err(|_| unavailable())?;
        let job = Job {
            id: id.clone(),
            handle: OwnedHandle(handle),
            creator: false,
        };
        job.check_limits(deadline)?;
        Ok(Self { job })
    }
    pub(crate) fn empty(&self, deadline: Instant) -> Result<bool> {
        Ok(self.job.members(deadline)?.is_empty())
    }
    /// Exactly one native call; successful return alone does not certify full process exit.
    pub(crate) fn request(&self, deadline: Instant) -> Result<()> {
        self.job.check_limits(deadline)?;
        // SAFETY: uniquely held QUERY|TERMINATE handle of the authenticated original job.
        // Explicit maintenance owns this whole job; no global PID/name matching or reassignment occurs.
        unsafe { TerminateJobObject(self.job.handle.0, 0x51434c49) }.map_err(|_| {
            AppError::new(
                "STOP_REQUEST_FAILED",
                "Native termination request failed; completion is unverified.",
                5,
            )
        })
    }
}

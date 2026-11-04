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
    /// A missing name alone is insufficient: a job can outlive its name.
    /// Legacy records can be reconciled across a reboot only when every recorded
    /// process predates this boot and none of those process generations survives.
    pub(crate) fn reopen_or_exited(
        id: &JobId,
        processes: &[crate::native::process::Identity],
        deadline: Instant,
    ) -> Result<Option<Self>> {
        crate::core::deadline::check(deadline)?;
        let name = id.wide()?;
        // SAFETY: authenticated name and owned noninheritable query handle.
        match unsafe { OpenJobObjectW(QUERY, false, PCWSTR(name.as_ptr())) } {
            Ok(handle) => {
                let _held = OwnedHandle(handle);
                Self::reopen(id, deadline).map(Some)
            }
            Err(error)
                if error.code()
                    == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND.to_hresult() =>
            {
                let now = u64::from(winsafe::GetSystemTimeAsFileTime());
                let uptime = winsafe::GetTickCount64()
                    .checked_mul(10_000)
                    .ok_or_else(unavailable)?;
                let boot = now.checked_sub(uptime).ok_or_else(unavailable)?;
                if processes.is_empty() {
                    return Err(unavailable());
                }
                for identity in processes {
                    identity.validate_for_host()?;
                    // Leave a minute around the boot boundary for clock/query resolution.
                    if identity
                        .created()
                        .checked_add(600_000_000)
                        .is_none_or(|created| created >= boot)
                    {
                        return Err(unavailable());
                    }
                    let process = winsafe::HPROCESS::OpenProcess(
                        winsafe::co::PROCESS::QUERY_LIMITED_INFORMATION
                            | winsafe::co::PROCESS::SYNCHRONIZE,
                        false,
                        identity.process_id(),
                    );
                    match process {
                        Err(e) if e == winsafe::co::ERROR::INVALID_PARAMETER => (),
                        Err(_) => return Err(unavailable()),
                        Ok(handle) => {
                            let (created, _, _, _) =
                                handle.GetProcessTimes().map_err(|_| unavailable())?;
                            if u64::from(created) == identity.created() {
                                return Err(unavailable());
                            }
                        }
                    }
                }
                crate::core::deadline::check(deadline)?;
                Ok(None)
            }
            Err(_) => Err(unavailable()),
        }
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

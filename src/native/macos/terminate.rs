//! Exact-process signals, not process-tree ownership. Lifecycle persists intent before dispatch.
use crate::core::error::{AppError, Result};
use crate::native::process::{Identity, Pinned};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Private lifecycle material. Deliberately no Debug or public-output adapter.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Target {
    schema_version: u8,
    identity: Identity,
    audit_token: [u32; 8],
}
unsafe extern "C" {
    fn qq_cli_capture_audit_token(pid: i32, output: *mut u32) -> i32;
    fn qq_cli_terminate_audit_token(input: *const u32) -> i32;
    fn qq_cli_suspend_audit_token(input: *const u32) -> i32;
    fn qq_cli_resume_audit_token(input: *const u32) -> i32;
}
impl Target {
    /// Caller must prove this process belongs to its run independently of this observation.
    /// No process is stopped or modified; only a read-only task-name right is acquired transiently.
    pub fn capture(owner: &Pinned, deadline: Instant) -> Result<Self> {
        owner.verify(deadline)?;
        let audit_token = capture(owner.identity().process_id())?;
        owner.verify(deadline)?;
        Ok(Self {
            schema_version: 1,
            identity: owner.identity().clone(),
            audit_token,
        })
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    /// Reopen original identity and audit generation; never refresh a stale target's authority.
    pub fn reopen(&self, deadline: Instant) -> Result<Pinned> {
        crate::core::deadline::check(deadline)?;
        if self.schema_version != 1 {
            return Err(conflict());
        }
        let owner = Pinned::reopen(&self.identity, deadline)?;
        self.verify(&owner, deadline)?;
        Ok(owner)
    }
    fn verify(&self, owner: &Pinned, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if self.schema_version != 1 || owner.identity() != &self.identity {
            return Err(conflict());
        }
        owner.verify(deadline)?;
        if capture(self.identity.process_id())? != self.audit_token {
            return Err(conflict());
        }
        owner.verify(deadline)
    }
    /// At most one kernel call; success means signal accepted, NEVER proof of stopped state.
    /// Caller has already persisted the exact target/intent and must not blindly replay on failure.
    pub fn terminate_once(&self, owner: &Pinned, deadline: Instant) -> Result<()> {
        self.signal_once(owner, deadline, Signal::Terminate)
    }
    /// Dispatch one SIGSTOP for the original generation, not proof of suspension.
    /// Lifecycle must persist the intent/recovery target first. Never suspend unrelated runs.
    pub fn suspend_once(&self, owner: &Pinned, deadline: Instant) -> Result<()> {
        self.signal_once(owner, deadline, Signal::Suspend)
    }
    /// Dispatch one SIGCONT for a suspension owned by this lifecycle operation.
    /// This does not prove running state and must not resume a user's preexisting stop.
    pub fn resume_once(&self, owner: &Pinned, deadline: Instant) -> Result<()> {
        self.signal_once(owner, deadline, Signal::Resume)
    }
    fn signal_once(&self, owner: &Pinned, deadline: Instant, signal: Signal) -> Result<()> {
        self.verify(owner, deadline)?;
        // SAFETY: eight initialized words in the SDK audit_token_t layout; the adapter retains no pointer.
        // The original generation is sent to the kernel even if the process exits after verify above.
        let result = unsafe {
            match signal {
                Signal::Terminate => qq_cli_terminate_audit_token(self.audit_token.as_ptr()),
                Signal::Suspend => qq_cli_suspend_audit_token(self.audit_token.as_ptr()),
                Signal::Resume => qq_cli_resume_audit_token(self.audit_token.as_ptr()),
            }
        };
        if result != 0 {
            return Err(AppError::new(
                match signal {
                    Signal::Terminate => "PROCESS_TERMINATION_REJECTED",
                    Signal::Suspend => "PROCESS_SUSPENSION_REJECTED",
                    Signal::Resume => "PROCESS_RESUMPTION_REJECTED",
                },
                "The kernel refused the original process target; no alternate PID was signalled.",
                4,
            ));
        }
        Ok(())
    }
}
enum Signal {
    Terminate,
    Suspend,
    Resume,
}
fn capture(pid: u32) -> Result<[u32; 8]> {
    let pid = i32::try_from(pid)
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(conflict)?;
    let mut token = [0_u32; 8];
    // SAFETY: adapter accepts a positive explicit PID, writes at most eight u32 values and releases its Mach right.
    if unsafe { qq_cli_capture_audit_token(pid, token.as_mut_ptr()) } != 0 {
        return Err(AppError::new(
            "PROCESS_TERMINATION_UNVERIFIABLE",
            "Original process termination identity could not be verified.",
            4,
        ));
    }
    Ok(token)
}
fn conflict() -> AppError {
    AppError::new(
        "PROCESS_TERMINATION_CONFLICT",
        "Original process termination identity has changed; no signal was sent.",
        4,
    )
}

#[cfg(test)]
#[path = "terminate/tests.rs"]
mod tests;

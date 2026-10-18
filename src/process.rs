//! Read-only process identity observations. Identity is not ownership or loaded-code proof.
use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Contains a private local path. Deliberately has no Debug or public-output adapter.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Identity {
    process_id: u32,
    /// Raw Windows FILETIME ticks, never a rounded wall-clock string.
    created: u64,
    image: PathBuf,
}
impl Identity {
    pub fn process_id(&self) -> u32 {
        self.process_id
    }
    pub fn created(&self) -> u64 {
        self.created
    }
    pub fn image(&self) -> &Path {
        &self.image
    }
    fn validate(&self) -> Result<()> {
        if self.process_id == 0
            || self.created == 0
            || !self.image.is_absolute()
            || self
                .image
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control))
        {
            return Err(changed());
        }
        Ok(())
    }
}
/// Owns a query/synchronization handle; dropping it closes the handle, not the process.
pub struct Pinned {
    identity: Identity,
    #[cfg(windows)]
    handle: winsafe::guard::CloseHandleGuard<winsafe::HPROCESS>,
}
impl Pinned {
    #[cfg(windows)]
    pub(crate) fn native_handle(&self) -> &winsafe::HPROCESS {
        &self.handle
    }
    /// The caller transfers an already-owned native process handle; no PID reopen.
    #[cfg(windows)]
    pub(crate) fn from_handle(
        handle: winsafe::guard::CloseHandleGuard<winsafe::HPROCESS>,
        deadline: Instant,
    ) -> Result<Self> {
        let identity = snapshot(&handle, deadline)?;
        Ok(Self { identity, handle })
    }
    /// Does not enumerate processes, inspect memory, acquire debug privilege or terminate.
    pub fn observe(process_id: u32, deadline: Instant) -> Result<Self> {
        crate::deadline::check(deadline)?;
        if process_id == 0 {
            return Err(AppError::invalid());
        }
        #[cfg(windows)]
        {
            let handle = winsafe::HPROCESS::OpenProcess(
                winsafe::co::PROCESS::QUERY_LIMITED_INFORMATION | winsafe::co::PROCESS::SYNCHRONIZE,
                false,
                process_id,
            )
            .map_err(|_| unavailable())?;
            let identity = snapshot(&handle, deadline)?;
            if identity.process_id != process_id {
                return Err(changed());
            }
            Ok(Self { identity, handle })
        }
        #[cfg(not(windows))]
        {
            Err(AppError::new(
                "PLATFORM_UNSUPPORTED",
                "Native process identity is supported only on Windows.",
                3,
            ))
        }
    }
    /// Reopening a numeric PID never grants trust: exact identity must still match.
    pub fn reopen(expected: &Identity, deadline: Instant) -> Result<Self> {
        crate::deadline::check(deadline)?;
        expected.validate()?;
        let process = Self::observe(expected.process_id, deadline)?;
        if &process.identity != expected {
            return Err(changed());
        }
        Ok(process)
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    /// Rechecks this SAME kernel process object; never silently reopens the PID.
    pub fn verify(&self, deadline: Instant) -> Result<()> {
        crate::deadline::check(deadline)?;
        #[cfg(windows)]
        {
            let observed = snapshot(&self.handle, deadline)?;
            if observed != self.identity {
                return Err(changed());
            }
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err(AppError::new(
                "PLATFORM_UNSUPPORTED",
                "Native process identity is supported only on Windows.",
                3,
            ))
        }
    }
}
#[cfg(windows)]
fn snapshot(handle: &winsafe::HPROCESS, deadline: Instant) -> Result<Identity> {
    alive(handle, deadline)?;
    let process_id = handle
        .GetProcessId()
        .map_err(|_| failed_observation(handle, deadline))?;
    crate::deadline::check(deadline)?;
    let (creation, _, _, _) = handle
        .GetProcessTimes()
        .map_err(|_| failed_observation(handle, deadline))?;
    crate::deadline::check(deadline)?;
    let image = handle
        .QueryFullProcessImageName(winsafe::co::PROCESS_NAME::WIN32)
        .map_err(|_| failed_observation(handle, deadline))?;
    let identity = Identity {
        process_id,
        created: u64::from(creation),
        image: image.into(),
    };
    identity.validate()?;
    alive(handle, deadline)?;
    Ok(identity)
}
#[cfg(windows)]
fn failed_observation(handle: &winsafe::HPROCESS, deadline: Instant) -> AppError {
    // Image metadata can disappear during teardown. Report an exit only when
    // this same kernel object is now signaled, never merely because a query failed.
    alive(handle, deadline).err().unwrap_or_else(unavailable)
}
#[cfg(windows)]
fn alive(handle: &winsafe::HPROCESS, deadline: Instant) -> Result<()> {
    crate::deadline::check(deadline)?;
    match handle
        .WaitForSingleObject(Some(0))
        .map_err(|_| unavailable())?
    {
        winsafe::co::WAIT::TIMEOUT => Ok(()),
        winsafe::co::WAIT::OBJECT_0 => Err(AppError::new(
            "PROCESS_EXITED",
            "The observed process has exited; no replacement was adopted.",
            4,
        )),
        _ => Err(unavailable()),
    }
}
#[cfg(windows)]
fn unavailable() -> AppError {
    AppError::new(
        "PROCESS_UNVERIFIABLE",
        "Process identity could not be verified; no ownership was inferred.",
        4,
    )
}
fn changed() -> AppError {
    AppError::new(
        "PROCESS_IDENTITY_CHANGED",
        "Process identity does not match its recorded creation and image; no replacement was adopted.",
        4,
    )
}

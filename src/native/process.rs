//! Read-only process identity observations. Identity is not ownership or loaded-code proof.
use crate::core::error::{AppError, Result};
use std::time::Instant;
mod identity;
pub use identity::Identity;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod current;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) use current::executable as current_executable;

/// A bounded observation of an exact executable file, not process-tree ownership.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use crate::native::macos_process::census::Executable;
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub use crate::native::macos_process::census::ExecutableSet;

/// Owns a native query handle or event subscription; dropping it never ends the process.
pub struct Pinned {
    identity: Identity,
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    watch: crate::native::macos_process::Watch,
    #[cfg(windows)]
    handle: winsafe::guard::CloseHandleGuard<winsafe::HPROCESS>,
}
impl Pinned {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub fn is_direct_child_of(&self, parent: &Self, deadline: Instant) -> Result<bool> {
        crate::native::macos_process::is_child(self, parent, deadline)
    }
    /// Observes SSTOP for the original macOS process, with identity checks on both sides.
    /// This is a point-in-time observation, not an exclusive freeze or process-tree proof.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub fn is_suspended(&self, deadline: Instant) -> Result<bool> {
        self.watch.is_suspended(&self.identity, deadline)
    }
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
        crate::core::deadline::check(deadline)?;
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
            if identity.process_id() != process_id {
                return Err(changed());
            }
            Ok(Self { identity, handle })
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            let (watch, identity) =
                crate::native::macos_process::Watch::observe(process_id, deadline)?;
            Ok(Self { identity, watch })
        }
        #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
        {
            Err(AppError::new(
                "PLATFORM_UNSUPPORTED",
                "Native process identity requires Windows or Apple Silicon macOS.",
                3,
            ))
        }
    }
    /// Reopening a numeric PID never grants trust: exact identity must still match.
    pub fn reopen(expected: &Identity, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        expected.validate_for_host()?;
        let process = Self::observe(expected.process_id(), deadline)?;
        if &process.identity != expected {
            return Err(changed());
        }
        Ok(process)
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    /// Rechecks the original handle/subscription and full identity; never adopts a replacement PID.
    pub fn verify(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        #[cfg(windows)]
        {
            let observed = snapshot(&self.handle, deadline)?;
            if observed != self.identity {
                return Err(changed());
            }
            Ok(())
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            self.watch.verify(&self.identity, deadline)
        }
        #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
        {
            Err(AppError::new(
                "PLATFORM_UNSUPPORTED",
                "Native process identity requires Windows or Apple Silicon macOS.",
                3,
            ))
        }
    }
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn exited(identity: &Identity, deadline: Instant) -> Result<bool> {
    crate::native::macos_process::exited(identity, deadline)
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn current_boot(identity: &Identity) -> Result<bool> {
    Ok(identity.macos_generation()?.2 == crate::native::macos_process::boot_session()?)
}
#[cfg(windows)]
fn snapshot(handle: &winsafe::HPROCESS, deadline: Instant) -> Result<Identity> {
    alive(handle, deadline)?;
    let process_id = handle
        .GetProcessId()
        .map_err(|_| failed_observation(handle, deadline))?;
    crate::core::deadline::check(deadline)?;
    let (creation, _, _, _) = handle
        .GetProcessTimes()
        .map_err(|_| failed_observation(handle, deadline))?;
    crate::core::deadline::check(deadline)?;
    let image = handle
        .QueryFullProcessImageName(winsafe::co::PROCESS_NAME::WIN32)
        .map_err(|_| failed_observation(handle, deadline))?;
    let identity = Identity::windows(process_id, u64::from(creation), image.into());
    identity.validate_for_host()?;
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
    crate::core::deadline::check(deadline)?;
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

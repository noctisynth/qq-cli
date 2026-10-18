//! Short, thread-bound native gate for credential API calls; never handles credential values.
#[cfg(test)]
mod tests;
use crate::core::error::{AppError, Result};
use std::{marker::PhantomData, rc::Rc};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0},
        System::Threading::{
            CreateMutexExW, MUTEX_MODIFY_STATE, ReleaseMutex, SYNCHRONIZATION_SYNCHRONIZE,
            WaitForSingleObject,
        },
    },
    core::PCWSTR,
};

pub(crate) struct Guard {
    handle: HANDLE,
    owned: bool,
    // Windows mutex ownership belongs to the calling thread; the guard must not be Send/Sync.
    _thread: PhantomData<Rc<()>>,
}
pub(crate) fn acquire() -> Result<Guard> {
    acquire_named("Local\\qq-cli-native-vault-v1", 1000)
}
fn open(name: &str) -> Result<Guard> {
    if !name.starts_with("Local\\qq-cli-") || name.len() > 200 || name.contains('\0') {
        return Err(unavailable());
    }
    let name = name.encode_utf16().chain([0]).collect::<Vec<_>>();
    // SAFETY: live terminated name, default token DACL, no inheritance or initial ownership.
    // Request only synchronization and release rights, never modify-DACL/owner rights.
    let handle = unsafe {
        CreateMutexExW(
            None,
            PCWSTR(name.as_ptr()),
            0,
            (SYNCHRONIZATION_SYNCHRONIZE | MUTEX_MODIFY_STATE).0,
        )
    }
    .map_err(|_| unavailable())?;
    Ok(Guard {
        handle,
        owned: false,
        _thread: PhantomData,
    })
}
fn acquire_named(name: &str, timeout: u32) -> Result<Guard> {
    let mut guard = open(name)?;
    // SAFETY: live uniquely-owned mutex handle; bounded wait, no native credential call yet.
    match unsafe { WaitForSingleObject(guard.handle, timeout) } {
        WAIT_OBJECT_0 => {
            guard.owned = true;
            Ok(guard)
        }
        WAIT_ABANDONED => {
            // The wait acquired ownership; release it on this same thread but do not use the vault.
            guard.owned = true;
            Err(unavailable())
        }
        _ => Err(unavailable()),
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: successful wait acquired one ownership count; !Send keeps release on its thread.
            let _ = unsafe { ReleaseMutex(self.handle) };
        }
        // SAFETY: unique successful creation/open handle; no transfer or inheritance occurred.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}
fn unavailable() -> AppError {
    AppError::new(
        "CREDENTIAL_UNAVAILABLE",
        "Protected credential access could not be coordinated.",
        4,
    )
}

//! Named jobs are private lifecycle capabilities, not account or loaded-code proof.
//! Reopening requires at least one surviving handle; member processes alone do not retain the name.
//! Only this module crosses the raw Win32 boundary. It never terminates a job.
mod retain;
mod spawn;
mod tcp;
use crate::core::error::{AppError, Result};
use crate::native::process::Pinned;
pub use retain::Retained;
use serde::{Deserialize, Serialize};
pub use spawn::{SpawnSpec, Spawned};
use std::time::Instant;
pub use tcp::BoundStream;
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, GetLastError, HANDLE, HLOCAL, LocalFree,
    SetLastError,
};
use windows::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::System::JobObjects::*;
use windows::core::{PCWSTR, w};

const PREFIX: &str = "Local\\qq-cli-";
const QUERY: u32 = 0x0004; // JOB_OBJECT_QUERY
const MAX_MEMBERS: usize = 256;
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(String);
impl JobId {
    pub(crate) fn validate(&self) -> Result<()> {
        self.wide().map(|_| ())
    }
    fn wide(&self) -> Result<Vec<u16>> {
        let suffix = self.0.strip_prefix(PREFIX).ok_or_else(unavailable)?;
        if suffix.len() != 64
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(unavailable());
        }
        Ok(self.0.encode_utf16().chain([0]).collect())
    }
}
struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: this wrapper uniquely owns a successful closeable Win32 object handle.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
fn pin(handle: OwnedHandle, deadline: Instant) -> Result<Pinned> {
    // SAFETY: move this unique process handle into winsafe's RAII owner exactly once.
    // The old owner's destructor is suppressed; the new owner closes on every subsequent error.
    let raw = std::mem::ManuallyDrop::new(handle).0;
    let handle =
        unsafe { winsafe::guard::CloseHandleGuard::new(winsafe::HPROCESS::from_ptr(raw.0)) };
    Pinned::from_handle(handle, deadline)
}
struct Descriptor(PSECURITY_DESCRIPTOR);
impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: conversion allocated this descriptor with LocalAlloc; free exactly once.
        let _ = unsafe { LocalFree(Some(HLOCAL(self.0.0))) };
    }
}
pub struct Job {
    id: JobId,
    handle: OwnedHandle,
    creator: bool,
}
impl Job {
    pub fn create(deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let mut nonce = [0u8; 32];
        getrandom::fill(&mut nonce).map_err(|_| unavailable())?;
        let id = JobId(format!(
            "{PREFIX}{}",
            nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        Self::create_named(id, deadline)
    }
    fn create_named(id: JobId, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let name = id.wide()?;
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: constant NUL-terminated SDDL, valid writable out pointer, no borrowed buffers escape.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                w!("D:P(A;;GA;;;SY)(A;;GA;;;OW)"),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )
        }
        .map_err(|_| unavailable())?;
        let descriptor = Descriptor(descriptor);
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0.0,
            bInheritHandle: false.into(),
        };
        // SAFETY: resetting this thread's last error avoids stale collision status;
        // attributes/descriptor and NUL-terminated name stay alive through this synchronous call.
        unsafe { SetLastError(ERROR_SUCCESS) };
        let handle = unsafe { CreateJobObjectW(Some(&attributes), PCWSTR(name.as_ptr())) }
            .map_err(|_| unavailable())?;
        // SAFETY: GetLastError is read immediately after successful creation, before any further Win32 call.
        let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let job = Self {
            id,
            handle: OwnedHandle(handle),
            creator: true,
        };
        if existed {
            return Err(unavailable());
        }
        job.check_limits(deadline)?;
        Ok(job)
    }
    /// Caller must authenticate this ID from its private lifecycle record first.
    pub fn reopen(id: &JobId, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let name = id.wide()?;
        // SAFETY: valid NUL-terminated buffer; returned query-only, noninheritable handle is uniquely owned.
        let handle = unsafe { OpenJobObjectW(QUERY, false, PCWSTR(name.as_ptr())) }
            .map_err(|_| unavailable())?;
        let job = Self {
            id: id.clone(),
            handle: OwnedHandle(handle),
            creator: false,
        };
        job.check_limits(deadline)?;
        Ok(job)
    }
    pub fn id(&self) -> &JobId {
        &self.id
    }
    fn check_limits(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        // SAFETY: owned live job handle and correctly sized/aligned writable struct, synchronous call.
        unsafe {
            QueryInformationJobObject(
                Some(self.handle.0),
                JobObjectExtendedLimitInformation,
                (&mut info as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
                None,
            )
        }
        .map_err(|_| unavailable())?;
        // No resource policy, breakaway, or kill-on-close is part of this lifecycle capability.
        if info.BasicLimitInformation.LimitFlags.0 != 0 {
            return Err(unavailable());
        }
        crate::core::deadline::check(deadline)
    }
    /// Snapshot only, bounded without global process enumeration. May be stale on return.
    pub fn members(&self, deadline: Instant) -> Result<Vec<u32>> {
        self.check_limits(deadline)?;
        // usize alignment supports the native flexible-array header and pointer-sized IDs.
        let mut storage = vec![0usize; 2 + MAX_MEMBERS];
        let bytes = std::mem::size_of_val(storage.as_slice());
        let mut returned = 0u32;
        // SAFETY: allocation is aligned and initialized; byte count describes its complete writable extent.
        unsafe {
            QueryInformationJobObject(
                Some(self.handle.0),
                JobObjectBasicProcessIdList,
                storage.as_mut_ptr().cast(),
                bytes as u32,
                Some(&mut returned),
            )
        }
        .map_err(|_| unavailable())?;
        // SAFETY: successful query initialized the two u32 header fields within this allocated buffer.
        let (assigned, count) = unsafe {
            let header = storage.as_ptr().cast::<u32>();
            (*header, *header.add(1) as usize)
        };
        let offset = std::mem::offset_of!(JOBOBJECT_BASIC_PROCESS_ID_LIST, ProcessIdList);
        if count > MAX_MEMBERS
            || assigned as usize != count
            || returned as usize > bytes
            || (returned as usize) < offset + count * std::mem::size_of::<usize>()
            || offset + count * std::mem::size_of::<usize>() > bytes
        {
            return Err(unavailable());
        }
        let mut result = Vec::with_capacity(count);
        for index in 0..count {
            // SAFETY: offset/alignment comes from the SDK struct; count and full array extent were checked above.
            let pid = unsafe {
                *storage
                    .as_ptr()
                    .cast::<u8>()
                    .add(offset)
                    .cast::<usize>()
                    .add(index)
            };
            let pid = u32::try_from(pid).map_err(|_| unavailable())?;
            if pid == 0 || result.contains(&pid) {
                return Err(unavailable());
            }
            result.push(pid);
        }
        crate::core::deadline::check(deadline)?;
        Ok(result)
    }
    pub fn contains(&self, process: &Pinned, deadline: Instant) -> Result<bool> {
        process.verify(deadline)?;
        let found = self
            .members(deadline)?
            .contains(&process.identity().process_id());
        process.verify(deadline)?;
        Ok(found)
    }
    #[cfg(test)]
    fn assign_test_child(&self, child: &Pinned) -> Result<()> {
        // SAFETY: borrowed live original creation handle; test keeps Pinned alive for this call.
        unsafe { AssignProcessToJobObject(self.handle.0, HANDLE(child.native_handle().ptr())) }
            .map_err(|_| unavailable())
    }
}
fn unavailable() -> AppError {
    AppError::new(
        "JOB_UNVERIFIABLE",
        "Private process job could not be verified; no process ownership was inferred.",
        4,
    )
}

#[cfg(test)]
mod tests;

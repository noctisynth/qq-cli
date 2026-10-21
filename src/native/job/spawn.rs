//! Synchronous atomic creation: all buffers and borrowed attribute values outlive the Win32 call.
use super::{Job, OwnedHandle, QUERY, unavailable};
use crate::core::error::{AppError, Result};
use crate::native::process::Pinned;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::time::Instant;
use windows::Win32::Foundation::{
    DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE,
};
use windows::Win32::System::Threading::*;
use windows::core::{PCWSTR, PWSTR};

/// Secret-bearing inputs have no Debug or Serialize. Shape checks are not startup trust gates.
pub struct SpawnSpec<'a> {
    pub program: &'a Path,
    pub arguments: &'a [OsString],
    pub environment: &'a BTreeMap<OsString, OsString>,
    pub directory: &'a Path,
}
impl SpawnSpec<'_> {
    /// Validate the complete native input shape before persisting a dispatch intent.
    /// Does not create a process, allocate a Job or expose environment values.
    pub fn validate(&self) -> Result<()> {
        if !local_path(self.program)
            || !self.program.is_file()
            || !local_path(self.directory)
            || !self.directory.is_dir()
        {
            return Err(invalid());
        }
        crate::storage::state::reject_links(self.program)?;
        crate::storage::state::reject_links(self.directory)?;
        wide(self.program.as_os_str())?;
        wide(self.directory.as_os_str())?;
        command(self.program.as_os_str(), self.arguments)?;
        environment(self.environment)?;
        Ok(())
    }
}
pub struct Spawned {
    process: Pinned,
}
impl Spawned {
    pub(crate) fn into_process(self) -> Pinned {
        self.process
    }
    pub fn process(&self) -> &Pinned {
        &self.process
    }
}
struct Attributes {
    _storage: Vec<u128>,
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
}
impl Attributes {
    fn new() -> Result<Self> {
        let mut size = 0usize;
        // SAFETY: documented sizing call, null list and valid out length. It is expected to fail.
        let sizing = unsafe { InitializeProcThreadAttributeList(None, 2, None, &mut size) };
        if sizing.is_ok() || size == 0 || size > 64 * 1024 {
            return Err(unavailable());
        }
        let mut storage = vec![0u128; size.div_ceil(16)];
        let pointer = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        // SAFETY: heap storage has 16-byte alignment, initialized bytes, and at least requested extent.
        unsafe { InitializeProcThreadAttributeList(Some(pointer), 2, None, &mut size) }
            .map_err(|_| unavailable())?;
        Ok(Self {
            _storage: storage,
            pointer,
        })
    }
}
impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: successfully initialized list; storage remains allocated until after this Drop body.
        unsafe { DeleteProcThreadAttributeList(self.pointer) };
    }
}
impl Job {
    /// Caller MUST persist the Job ID/start intent and check trust gates before calling.
    /// An inherited query-only handle keeps this ROOT process's job name alive.
    /// Retention across a noncooperating loader's descendants is not proved here.
    pub fn spawn(&self, spec: &SpawnSpec<'_>, deadline: Instant) -> Result<Spawned> {
        crate::core::deadline::check(deadline)?;
        if !self.creator {
            return Err(unavailable());
        }
        self.check_limits(deadline)?;
        spec.validate()?;
        let application = wide(spec.program.as_os_str())?;
        let directory = wide(spec.directory.as_os_str())?;
        let mut command = command(spec.program.as_os_str(), spec.arguments)?;
        let environment = environment(spec.environment)?;
        let mut inherited = HANDLE::default();
        // SAFETY: valid current-process pseudo handles, borrowed owned job handle, writable out handle;
        // duplication grants QUERY only. The resulting owned handle is intentionally inheritable.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                self.handle.0,
                GetCurrentProcess(),
                &mut inherited,
                QUERY,
                true,
                DUPLICATE_HANDLE_OPTIONS(0),
            )
        }
        .map_err(|_| unavailable())?;
        let inherited = OwnedHandle(inherited);
        let null = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(r"\\.\NUL")
            .map_err(|_| unavailable())?;
        use std::os::windows::io::AsRawHandle;
        let mut null_inherited = HANDLE::default();
        // SAFETY: live borrowed File handle; duplicate into the current process as inheritable.
        // SAME_ACCESS retains only this NUL device's read/write rights, not any parent stream.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(null.as_raw_handle()),
                GetCurrentProcess(),
                &mut null_inherited,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(|_| unavailable())?;
        let null_inherited = OwnedHandle(null_inherited);
        let jobs = [self.handle.0];
        let handles = [inherited.0, null_inherited.0];
        // Declare attributes LAST: arrays and handle owners outlive attribute deletion on every return path.
        let attributes = Attributes::new()?;
        // SAFETY: initialized list and live HANDLE arrays; both arrays remain stable and alive
        // until attributes is deleted, including through synchronous CreateProcessW below.
        unsafe {
            UpdateProcThreadAttribute(
                attributes.pointer,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                Some(jobs.as_ptr().cast()),
                std::mem::size_of_val(&jobs),
                None,
                None,
            )
            .map_err(|_| unavailable())?;
            UpdateProcThreadAttribute(
                attributes.pointer,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                Some(handles.as_ptr().cast()),
                std::mem::size_of_val(&handles),
                None,
                None,
            )
        }
        .map_err(|_| unavailable())?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = null_inherited.0;
        startup.StartupInfo.hStdOutput = null_inherited.0;
        startup.StartupInfo.hStdError = null_inherited.0;
        // All three streams explicitly use NUL; no token log or parent stream is forwarded.
        startup.lpAttributeList = attributes.pointer;
        let mut info = PROCESS_INFORMATION::default();
        crate::core::deadline::check(deadline)?;
        // SAFETY: absolute explicit image, writable command, double-NUL owned environment, stable cwd,
        // initialized extended startup and out info. All buffers/handles/attributes outlive this call.
        unsafe {
            CreateProcessW(
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command.as_mut_ptr())),
                None,
                None,
                true,
                CREATE_UNICODE_ENVIRONMENT | EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW,
                Some(environment.as_ptr().cast()),
                PCWSTR(directory.as_ptr()),
                &startup.StartupInfo,
                &mut info,
            )
        }
        .map_err(|_| {
            AppError::new(
                "START_FAILED",
                "Native process creation failed; no successful creation was reported.",
                3,
            )
        })?;
        let process = OwnedHandle(info.hProcess);
        let _thread = OwnedHandle(info.hThread);
        let process = super::pin(process, deadline).map_err(|error| {
            #[cfg(test)]
            eprintln!("synthetic original process observation failed: {error:?}");
            let _ = error;
            unknown()
        })?;
        if process.identity().process_id() != info.dwProcessId
            || !self.contains(&process, deadline).map_err(|_| unknown())?
        {
            return Err(unknown());
        }
        Ok(Spawned { process })
    }
}
fn wide(value: &OsStr) -> Result<Vec<u16>> {
    let value: Vec<u16> = value.encode_wide().collect();
    if value.contains(&0) || value.len() > 32766 {
        return Err(invalid());
    }
    Ok(value.into_iter().chain([0]).collect())
}
fn command(program: &OsStr, arguments: &[OsString]) -> Result<Vec<u16>> {
    if arguments.len() > 128 {
        return Err(invalid());
    }
    let mut output = Vec::new();
    for (i, value) in std::iter::once(program)
        .chain(arguments.iter().map(OsString::as_os_str))
        .enumerate()
    {
        if i > 0 {
            output.push(b' ' as u16);
        }
        output.push(b'"' as u16);
        let value = wide(value)?;
        let mut slashes = 0;
        for &unit in &value[..value.len() - 1] {
            if unit == b'\\' as u16 {
                slashes += 1;
                continue;
            }
            output.extend(std::iter::repeat_n(
                b'\\' as u16,
                if unit == b'"' as u16 {
                    slashes * 2 + 1
                } else {
                    slashes
                },
            ));
            output.push(unit);
            slashes = 0;
        }
        output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
        output.push(b'"' as u16);
        if output.len() > 32766 {
            return Err(invalid());
        }
    }
    output.push(0);
    Ok(output)
}
fn environment(values: &BTreeMap<OsString, OsString>) -> Result<Vec<u16>> {
    if values.len() > 128 {
        return Err(invalid());
    }
    let mut output = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for (name, value) in values {
        let name = name.to_str().ok_or_else(invalid)?;
        if name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'(' | b')'))
            || !seen.insert(name.to_ascii_uppercase())
        {
            return Err(invalid());
        }
        entries.push((name.to_ascii_uppercase(), value));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, value) in entries {
        output.extend(name.encode_utf16());
        output.push(b'=' as u16);
        let value = wide(value)?;
        output.extend(value);
        if output.len() > 32766 {
            return Err(invalid());
        }
    }
    if output.is_empty() {
        output.push(0);
    }
    output.push(0);
    Ok(output)
}
fn local_path(path: &Path) -> bool {
    let units: Vec<u16> = path.as_os_str().encode_wide().take(7).collect();
    let drive = |s: &[u16]| {
        s.len() >= 3
            && s[0] <= 127
            && (s[0] as u8).is_ascii_alphabetic()
            && s[1] == b':' as u16
            && matches!(s[2], 47 | 92)
    };
    drive(&units) || (units.starts_with(&[92, 92, 63, 92]) && drive(&units[4..]))
}
fn invalid() -> AppError {
    AppError::new(
        "LAUNCH_INPUT_INVALID",
        "Native process inputs are invalid; no process was started.",
        2,
    )
}
fn unknown() -> AppError {
    let mut error = AppError::new(
        "START_OUTCOME_UNKNOWN",
        "A process was created but verification did not complete; do not start another automatically.",
        5,
    );
    error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
    error
}

#[cfg(test)]
mod tests;

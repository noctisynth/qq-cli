//! Synchronous atomic creation: all buffers and borrowed attribute values outlive the Win32 call.
use super::{Job, OwnedHandle, QUERY, unavailable};
use crate::error::{AppError, Result};
use crate::process::Pinned;
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
        crate::deadline::check(deadline)?;
        if !self.creator {
            return Err(unavailable());
        }
        self.check_limits(deadline)?;
        if !local_path(spec.program)
            || !spec.program.is_file()
            || !local_path(spec.directory)
            || !spec.directory.is_dir()
        {
            return Err(invalid());
        }
        crate::state::reject_links(spec.program)?;
        crate::state::reject_links(spec.directory)?;
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
        crate::deadline::check(deadline)?;
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
        let process = super::pin(process, deadline).map_err(|_| unknown())?;
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
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
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
    error.outcome = Some("unknown");
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }
    #[test]
    fn command_and_environment_encoding_obey_boundaries_without_inheriting_defaults() {
        assert_eq!(
            String::from_utf16(
                &command(
                    OsStr::new("original.exe"),
                    &[
                        "".into(),
                        "space value".into(),
                        "quote\"value".into(),
                        "tail\\".into()
                    ]
                )
                .unwrap()
            )
            .unwrap(),
            "\"original.exe\" \"\" \"space value\" \"quote\\\"value\" \"tail\\\\\"\0"
        );
        assert_eq!(environment(&BTreeMap::new()).unwrap(), vec![0, 0]);
        let values = BTreeMap::from([
            ("z_key".into(), "值".into()),
            ("A_KEY".into(), "space value".into()),
        ]);
        assert_eq!(
            String::from_utf16(&environment(&values).unwrap()).unwrap(),
            "A_KEY=space value\0Z_KEY=值\0\0"
        );
        for values in [
            BTreeMap::from([("same".into(), "one".into()), ("SAME".into(), "two".into())]),
            BTreeMap::from([("bad=name".into(), "value".into())]),
            BTreeMap::from([("KEY".into(), "embedded\0value".into())]),
        ] {
            assert!(environment(&values).is_err());
        }
        assert!(command(OsStr::new("original.exe"), &["x".repeat(32767).into()]).is_err());
        assert!(command(OsStr::new("original.exe"), &vec![OsString::new(); 129]).is_err());
        assert!(environment(&BTreeMap::from([("KEY".into(), "x".repeat(32767).into())])).is_err());
        for path in [
            r"\\server\share\original.exe",
            r"\\.\device",
            r"\relative",
            r"relative",
        ] {
            assert!(!local_path(Path::new(path)));
        }
    }

    #[test]
    fn invalid_expired_and_query_only_launches_never_start_a_program() {
        let temp = tempfile::tempdir().unwrap();
        let executable = std::env::current_exe().unwrap();
        let job = Job::create(deadline()).unwrap();
        let spec = SpawnSpec {
            program: &executable,
            arguments: &[],
            environment: &BTreeMap::new(),
            directory: temp.path(),
        };
        assert_eq!(
            job.spawn(&spec, Instant::now()).err().unwrap().code,
            "DEADLINE"
        );
        let queried = Job::reopen(job.id(), deadline()).unwrap();
        assert_eq!(
            queried.spawn(&spec, deadline()).err().unwrap().code,
            "JOB_UNVERIFIABLE"
        );
        let invalid_env = BTreeMap::from([("bad=name".into(), "synthetic".into())]);
        assert_eq!(
            job.spawn(
                &SpawnSpec {
                    environment: &invalid_env,
                    ..spec
                },
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "LAUNCH_INPUT_INVALID"
        );
        let relative = std::path::PathBuf::from("relative-original.exe");
        assert_eq!(
            job.spawn(
                &SpawnSpec {
                    program: &relative,
                    ..spec
                },
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "LAUNCH_INPUT_INVALID"
        );
        let not_executable = temp.path().join("original-invalid.exe");
        std::fs::write(&not_executable, b"original non-executable synthetic input").unwrap();
        assert_eq!(
            job.spawn(
                &SpawnSpec {
                    program: &not_executable,
                    ..spec
                },
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "START_FAILED"
        );
        assert!(job.members(deadline()).unwrap().is_empty());
    }

    #[test]
    fn atomic_creation_preserves_arguments_environment_and_root_job_name_after_parent_handles_close()
     {
        const MODE: &str = "QQ_CLI_TEST_ATOMIC_MODE";
        if std::env::var_os(MODE).is_some() {
            let root =
                std::path::PathBuf::from(std::env::var_os("QQ_CLI_TEST_ATOMIC_ROOT").unwrap());
            let id: super::super::JobId =
                serde_json::from_str(&std::env::var("QQ_CLI_TEST_ATOMIC_JOB").unwrap()).unwrap();
            let job = Job::reopen(&id, deadline()).unwrap();
            assert!(
                job.members(deadline())
                    .unwrap()
                    .contains(&std::process::id())
            );
            drop(job); // Only the inherited raw QUERY handle retains the name from here.
            let args: Vec<String> = std::env::args().collect();
            use std::os::windows::io::AsRawHandle;
            std::fs::write(
                root.join("startup.json"),
                serde_json::to_vec(&serde_json::json!({
                    "cwdMatches":std::env::current_dir().unwrap()==root,
                    "sameNullStream":std::io::stdin().as_raw_handle()==std::io::stdout().as_raw_handle()
                        && std::io::stdout().as_raw_handle()==std::io::stderr().as_raw_handle()
                }))
                .unwrap(),
            )
            .unwrap();
            assert_eq!(std::env::current_dir().unwrap(), root);
            assert_eq!(
                std::io::stdin().as_raw_handle(),
                std::io::stdout().as_raw_handle()
            );
            assert_eq!(
                std::io::stdout().as_raw_handle(),
                std::io::stderr().as_raw_handle()
            );
            use std::io::{Read, Write};
            assert_eq!(std::io::stdin().read(&mut [0]).unwrap(), 0);
            std::io::stdout()
                .write_all(b"original-null-stream-probe")
                .unwrap();
            std::io::stderr()
                .write_all(b"original-null-stream-probe")
                .unwrap();
            assert!(std::env::var_os("PATH").is_none());
            assert_eq!(
                std::env::var("QQ_CLI_TEST_ATOMIC_VALUE").unwrap(),
                "原创 synthetic value"
            );
            std::fs::write(root.join("report.json"), serde_json::to_vec(&args).unwrap()).unwrap();
            let until = Instant::now() + Duration::from_secs(15);
            while !root.join("release").exists() && Instant::now() < until {
                std::thread::sleep(Duration::from_millis(10));
            }
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("original space 空间");
        std::fs::create_dir(&root).unwrap();
        struct Release(std::path::PathBuf);
        impl Drop for Release {
            fn drop(&mut self) {
                let _ = std::fs::write(&self.0, b"release original child");
            }
        }
        let release = Release(root.join("release"));
        let job = Job::create(deadline()).unwrap();
        let id = job.id().clone();
        let mut environment = BTreeMap::from([
            (MODE.into(), "1".into()),
            (
                "QQ_CLI_TEST_ATOMIC_ROOT".into(),
                root.clone().into_os_string(),
            ),
            (
                "QQ_CLI_TEST_ATOMIC_JOB".into(),
                serde_json::to_string(&id).unwrap().into(),
            ),
            (
                "QQ_CLI_TEST_ATOMIC_VALUE".into(),
                "原创 synthetic value".into(),
            ),
        ]);
        if let Some(system) = std::env::var_os("SystemRoot") {
            environment.insert("SYSTEMROOT".into(), system);
        }
        let executable = std::env::current_exe().unwrap();
        let arguments: Vec<OsString> = ["--exact","native_job::spawn::tests::atomic_creation_preserves_arguments_environment_and_root_job_name_after_parent_handles_close",
            "--test-threads=1","--","", "space value", "quote\"value", "tail\\", "原创"].into_iter().map(Into::into).collect();
        // Extra arguments after -- are test filters, but remain observable through std::env::args.
        let spawned = job
            .spawn(
                &SpawnSpec {
                    program: &executable,
                    arguments: &arguments,
                    environment: &environment,
                    directory: &root,
                },
                deadline(),
            )
            .unwrap();
        let until = deadline();
        while !root.join("report.json").exists() && Instant::now() < until {
            if spawned.process().verify(deadline()).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            root.join("report.json").exists(),
            "original startup observation: {}",
            std::fs::read_to_string(root.join("startup.json"))
                .unwrap_or_else(|_| "no startup observation".into())
        );
        let report: Vec<String> =
            serde_json::from_slice(&std::fs::read(root.join("report.json")).unwrap()).unwrap();
        assert_eq!(
            report[1..],
            arguments
                .iter()
                .map(|a| a.to_str().unwrap().to_owned())
                .collect::<Vec<_>>()
        );
        assert!(job.contains(spawned.process(), deadline()).unwrap());
        drop(job);
        let reopened = Job::reopen(&id, deadline()).unwrap();
        assert!(reopened.contains(spawned.process(), deadline()).unwrap());
        drop(reopened);
        drop(release);
        let until = deadline();
        while Instant::now() < until {
            match spawned.process().verify(deadline()) {
                Err(e) if e.code == "PROCESS_EXITED" => break,
                Err(e) if e.code != "PROCESS_UNVERIFIABLE" => {
                    panic!("original child observation: {}", e.code)
                }
                _ => (),
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            spawned.process().verify(deadline()).unwrap_err().code,
            "PROCESS_EXITED"
        );
        let until = deadline();
        let mut observations = 0;
        let mut stale_members = 0;
        loop {
            let query = match Job::reopen(&id, until) {
                Ok(query) => query,
                Err(error) => {
                    assert_eq!(error.code, "JOB_UNVERIFIABLE");
                    break;
                }
            };
            // Process signaling and namespace cleanup are separate observations.
            // Drop each query before waiting so the observer does not retain the name.
            let members = query.members(until).unwrap();
            let unexpected: Vec<_> = members
                .iter()
                .filter(|pid| **pid != spawned.process().identity().process_id())
                .map(|pid| {
                    (
                        *pid,
                        Pinned::observe(*pid, until)
                            .map(|p| {
                                p.identity()
                                    .image()
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned()
                            })
                            .map_err(|e| e.code),
                    )
                })
                .collect();
            // Observed Windows conhost can outlive the test root briefly in this Job.
            // The root's verified exit does not prove every descendant has exited.
            if !unexpected.is_empty() {
                println!("original post-exit auxiliary observations: {unexpected:?}");
            }
            if !members.is_empty() {
                stale_members += 1;
            }
            drop(query);
            observations += 1;
            assert!(
                Instant::now() < until,
                "original Job name remained after root exit"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("original post-exit Job cleanup observations: {observations}");
        println!("original post-exit stale-member observations: {stale_members}");
    }
}

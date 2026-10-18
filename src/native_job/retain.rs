//! Retain one query-only job handle in a verified member's handle table, without executing code there.
use super::{Job, JobId, OwnedHandle, QUERY, pin, unavailable};
use crate::error::{AppError, Result};
use crate::process::{Identity, Pinned};
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
        crate::deadline::check(deadline)?;
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
        crate::deadline::check(deadline)?;
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
    crate::deadline::check(deadline)?;
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
    error.outcome = Some("unknown");
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_job::SpawnSpec;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use windows::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT, HANDLE_FLAGS};
    use windows::Win32::System::JobObjects::AssignProcessToJobObject;
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }
    const NAME: &str = "native_job::retain::tests::verified_descendant_keeps_readonly_job_after_original_loader_exits";
    const MODE: &str = "QQ_CLI_TEST_RETAIN_ROLE";
    const ROOT: &str = "QQ_CLI_TEST_RETAIN_ROOT";
    const ID: &str = "QQ_CLI_TEST_RETAIN_JOB";
    fn wait_file(path: &Path) {
        let until = deadline();
        while !path.exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(path.exists(), "original process handshake timed out");
    }
    fn wait_exit(process: &Pinned) {
        let until = deadline();
        while Instant::now() < until {
            match process.verify(deadline()) {
                Err(e) if e.code == "PROCESS_EXITED" => return,
                Err(e) if e.code != "PROCESS_UNVERIFIABLE" => {
                    panic!("original observation: {}", e.code)
                }
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        panic!("original process did not exit");
    }
    #[test]
    fn verified_descendant_keeps_readonly_job_after_original_loader_exits() {
        if let Some(role) = std::env::var_os(MODE) {
            let root = PathBuf::from(std::env::var_os(ROOT).unwrap());
            let id: JobId = serde_json::from_str(&std::env::var(ID).unwrap()).unwrap();
            if role == "loader" {
                // Explicit HANDLE_LIST excludes the ORIGINAL root job handle.
                // A nested original job is inherited instead, so the no-retain
                // control cannot accidentally pass through broad legacy inheritance.
                let nested = Job::create(deadline()).unwrap();
                let mut environment: BTreeMap<OsString, OsString> = BTreeMap::from([
                    (MODE.into(), "worker".into()),
                    (ROOT.into(), root.as_os_str().to_owned()),
                    (ID.into(), serde_json::to_string(&id).unwrap().into()),
                ]);
                if let Some(system) = std::env::var_os("SystemRoot") {
                    environment.insert("SYSTEMROOT".into(), system);
                }
                let executable = std::env::current_exe().unwrap();
                let arguments = ["--exact".into(), NAME.into(), "--test-threads=1".into()];
                let _child = nested
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
                let until = Instant::now() + Duration::from_secs(15);
                while !root.join("release-loader").exists() && Instant::now() < until {
                    std::thread::sleep(Duration::from_millis(10));
                }
                return;
            }
            assert_eq!(role, "worker");
            let process = Pinned::observe(std::process::id(), deadline()).unwrap();
            {
                let query = Job::reopen(&id, deadline()).unwrap();
                assert!(query.contains(&process, deadline()).unwrap());
            }
            crate::state::atomic_json(&root.join("worker.json"), process.identity()).unwrap();
            let until = Instant::now() + Duration::from_secs(15);
            while !root.join("release-worker").exists() && Instant::now() < until {
                let receipt = root.join("receipt.json");
                if receipt.exists() && !root.join("confirmed.json").exists() {
                    let receipt: Retained = crate::config::read_json(&receipt, 4096).unwrap();
                    assert_eq!(receipt.target.process_id(), std::process::id());
                    assert_eq!(receipt.schema_version, 1);
                    let remote = HANDLE(receipt.remote_handle as usize as *mut std::ffi::c_void);
                    let mut found = false.into();
                    let mut flags = HANDLE_FLAGS::default();
                    // SAFETY: this test's parent just copied this exact handle into THIS process;
                    // receipt is in the isolated original fixture directory, and both out structs are live.
                    unsafe {
                        IsProcessInJob(GetCurrentProcess(), Some(remote), &mut found).unwrap();
                        GetHandleInformation(remote, &mut flags.0).unwrap();
                    }
                    assert!(found.as_bool());
                    assert_eq!(flags.0 & HANDLE_FLAG_INHERIT.0, 0);
                    // SAFETY: same live query handle and current-process pseudo handle. The operation
                    // must fail for missing ASSIGN permission; no limits or memory are modified.
                    assert!(
                        unsafe { AssignProcessToJobObject(remote, GetCurrentProcess()) }.is_err()
                    );
                    crate::state::atomic_json(
                        &root.join("confirmed.json"),
                        &serde_json::json!({"queryOnly":true,"inheritable":false}),
                    )
                    .unwrap();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            return;
        }
        for retain in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path();
            struct Release(PathBuf);
            impl Drop for Release {
                fn drop(&mut self) {
                    let _ = std::fs::write(self.0.join("release-loader"), b"original release");
                    let _ = std::fs::write(self.0.join("release-worker"), b"original release");
                }
            }
            let release = Release(root.to_owned());
            let job = Job::create(deadline()).unwrap();
            let id = job.id().clone();
            let mut environment: BTreeMap<OsString, OsString> = BTreeMap::from([
                (MODE.into(), "loader".into()),
                (ROOT.into(), root.as_os_str().to_owned()),
                (ID.into(), serde_json::to_string(&id).unwrap().into()),
            ]);
            if let Some(system) = std::env::var_os("SystemRoot") {
                environment.insert("SYSTEMROOT".into(), system);
            }
            let executable = std::env::current_exe().unwrap();
            let arguments = ["--exact".into(), NAME.into(), "--test-threads=1".into()];
            let loader = job
                .spawn(
                    &SpawnSpec {
                        program: &executable,
                        arguments: &arguments,
                        environment: &environment,
                        directory: root,
                    },
                    deadline(),
                )
                .unwrap();
            wait_file(&root.join("worker.json"));
            let identity: Identity =
                crate::config::read_json(&root.join("worker.json"), 4096).unwrap();
            assert!(
                job.members(deadline())
                    .unwrap()
                    .contains(&identity.process_id())
            );
            let worker = Pinned::reopen(&identity, deadline()).unwrap();
            if retain {
                let store = crate::state::Store::new(root.join("original-state"));
                let references = crate::references::References::with_key(
                    store.root().join("references"),
                    [46; 32],
                );
                let mut tx = store.transaction(deadline()).unwrap();
                tx.registry.instances.push(crate::state::Instance {
                    id: "original-instance".into(),
                    installation_id: "unused-original".into(),
                    endpoint: "http://127.0.0.1:1/".into(),
                    ws_endpoint: None,
                    credential_key: "unused-original".into(),
                    account: None,
                    run_id: "original-run".into(),
                    state: "starting".into(),
                    process_id: None,
                    process_started: None,
                });
                tx.persist().unwrap();
                let report = crate::retention::retain_once(
                    &tx,
                    &references,
                    "original-instance",
                    "original-run",
                    &job,
                    &worker,
                    deadline(),
                )
                .unwrap();
                assert_eq!(report.outcome(), crate::retention::Outcome::Retained);
                let receipt = report.receipt().unwrap().clone();
                let cached = crate::retention::retain_once(
                    &tx,
                    &references,
                    "original-instance",
                    "original-run",
                    &job,
                    &worker,
                    deadline(),
                )
                .unwrap();
                assert_eq!(
                    serde_json::to_value(cached.receipt()).unwrap(),
                    serde_json::to_value(&receipt).unwrap()
                );
                assert!(receipt.target() == &identity);
                crate::state::atomic_json(&root.join("receipt.json"), &receipt).unwrap();
                wait_file(&root.join("confirmed.json"));
            }
            std::fs::write(root.join("release-loader"), b"original release").unwrap();
            wait_exit(loader.process());
            drop(job);
            worker.verify(deadline()).unwrap();
            if retain {
                let query = Job::reopen(&id, deadline()).unwrap();
                assert!(query.contains(&worker, deadline()).unwrap());
                drop(query);
            } else {
                assert!(Job::reopen(&id, deadline()).is_err());
            }
            drop(release);
            wait_exit(&worker);
            assert!(Job::reopen(&id, deadline()).is_err());
        }
    }
    #[test]
    fn foreign_and_expired_targets_do_not_receive_handles() {
        let job = Job::create(deadline()).unwrap();
        let own_test_process = Pinned::observe(std::process::id(), deadline()).unwrap();
        assert_eq!(
            job.retain_in(&own_test_process, Instant::now())
                .err()
                .unwrap()
                .code,
            "DEADLINE"
        );
        assert_eq!(
            job.retain_in(&own_test_process, deadline())
                .err()
                .unwrap()
                .code,
            "JOB_MEMBER_MISMATCH"
        );
        assert!(job.members(deadline()).unwrap().is_empty());
    }
}

use super::*;
use crate::native::job::SpawnSpec;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use windows::Win32::Foundation::{GetHandleInformation, HANDLE_FLAG_INHERIT, HANDLE_FLAGS};
use windows::Win32::System::JobObjects::AssignProcessToJobObject;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}
const NAME: &str = "native::job::retain::tests::verified_descendant_keeps_readonly_job_after_original_loader_exits";
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
        crate::storage::state::atomic_json(&root.join("worker.json"), process.identity()).unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        while !root.join("release-worker").exists() && Instant::now() < until {
            let receipt = root.join("receipt.json");
            if receipt.exists() && !root.join("confirmed.json").exists() {
                let receipt: Retained = crate::storage::config::read_json(&receipt, 4096).unwrap();
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
                assert!(unsafe { AssignProcessToJobObject(remote, GetCurrentProcess()) }.is_err());
                crate::storage::state::atomic_json(
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
            crate::storage::config::read_json(&root.join("worker.json"), 4096).unwrap();
        assert!(
            job.members(deadline())
                .unwrap()
                .contains(&identity.process_id())
        );
        let worker = Pinned::reopen(&identity, deadline()).unwrap();
        if retain {
            let store = crate::storage::state::Store::new(root.join("original-state"));
            let references = crate::storage::references::References::with_key(
                store.root().join("references"),
                [46; 32],
            );
            let mut tx = store.transaction(deadline()).unwrap();
            tx.registry.instances.push(crate::storage::state::Instance {
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
            let report = crate::napcat::runtime::retention::retain_once(
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
                report.outcome(),
                crate::napcat::runtime::retention::Outcome::Retained
            );
            let receipt = report.receipt().unwrap().clone();
            let cached = crate::napcat::runtime::retention::retain_once(
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
            crate::storage::state::atomic_json(&root.join("receipt.json"), &receipt).unwrap();
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

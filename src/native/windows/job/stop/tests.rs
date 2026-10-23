use super::*;
use crate::native::job::SpawnSpec;
use std::{collections::BTreeMap, ffi::OsString, time::Duration};

fn end() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[test]
fn explicit_stop_only_ends_its_original_job_and_query_handles_cannot_stop() {
    const MODE: &str = "QQ_CLI_TEST_EXPLICIT_JOB_STOP";
    if let Some(root) = std::env::var_os(MODE) {
        let root = std::path::PathBuf::from(root);
        let until = Instant::now() + Duration::from_secs(30);
        while !root.join("release").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    struct Release(std::path::PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, b"release original fixture");
        }
    }
    let release = Release(temp.path().join("release"));
    let mut environment =
        BTreeMap::from([(OsString::from(MODE), temp.path().as_os_str().to_owned())]);
    if let Some(system) = std::env::var_os("SystemRoot") {
        environment.insert("SYSTEMROOT".into(), system);
    }
    let program = std::env::current_exe().unwrap();
    let arguments = ["--exact".into(), "native::job::stop::tests::explicit_stop_only_ends_its_original_job_and_query_handles_cannot_stop".into(), "--test-threads=1".into()];
    let spec = SpawnSpec {
        program: &program,
        arguments: &arguments,
        environment: &environment,
        directory: temp.path(),
    };
    let first = Job::create(end()).unwrap();
    let other = Job::create(end()).unwrap();
    let first_child = first.spawn(&spec, end()).unwrap();
    let other_child = other.spawn(&spec, end()).unwrap();
    let query = Job::reopen(first.id(), end()).unwrap();
    // SAFETY: our own original empty-wait child group; QUERY is expected to deny termination.
    assert!(unsafe { TerminateJobObject(query.handle.0, 1) }.is_err());
    first_child.process().verify(end()).unwrap();
    let maintenance = StopJob::reopen(first.id(), end()).unwrap();
    drop(maintenance);
    first_child.process().verify(end()).unwrap();
    let expired = Instant::now() - Duration::from_millis(1);
    assert!(StopJob::reopen(first.id(), expired).is_err());
    let maintenance = StopJob::reopen(first.id(), end()).unwrap();
    assert!(maintenance.request(expired).is_err());
    first_child.process().verify(end()).unwrap();
    maintenance.request(end()).unwrap();
    let until = end();
    while !maintenance.empty(end()).unwrap() {
        assert!(Instant::now() < until, "original stop fixture timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Job accounting may become empty before the individual process object is signaled.
    let until = end();
    loop {
        match first_child.process().verify(end()) {
            Err(error) if error.code == "PROCESS_EXITED" => break,
            Ok(()) => (),
            Err(error) => panic!("unexpected original process observation: {}", error.code),
        }
        assert!(
            Instant::now() < until,
            "original process teardown timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    other_child.process().verify(end()).unwrap();
    drop(release);
    let until = end();
    while other_child.process().verify(end()).is_ok() {
        assert!(
            Instant::now() < until,
            "original cooperative fixture timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

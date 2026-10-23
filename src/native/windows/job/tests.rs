use super::*;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::time::Duration;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[test]
fn named_job_reopen_requires_a_live_handle_and_closing_handles_keeps_child_alive() {
    const MODE: &str = "QQ_CLI_TEST_NAMED_JOB_CHILD";
    if let Some(root) = std::env::var_os(MODE) {
        let root = std::path::PathBuf::from(root);
        let until = Instant::now() + Duration::from_secs(15);
        while !root.join("release").exists() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    struct Release(std::path::PathBuf);
    impl Drop for Release {
        fn drop(&mut self) {
            let _ = std::fs::write(&self.0, b"original release");
        }
    }
    let release = Release(temp.path().join("release"));
    let guard_job = Job::create(deadline()).unwrap();
    let mut environment =
        BTreeMap::from([(OsString::from(MODE), temp.path().as_os_str().to_owned())]);
    if let Some(system) = std::env::var_os("SystemRoot") {
        environment.insert("SYSTEMROOT".into(), system);
    }
    let executable = std::env::current_exe().unwrap();
    let arguments = ["--exact".into(),"native::job::tests::named_job_reopen_requires_a_live_handle_and_closing_handles_keeps_child_alive".into(),"--test-threads=1".into()];
    let child = guard_job
        .spawn(
            &SpawnSpec {
                program: &executable,
                arguments: &arguments,
                environment: &environment,
                directory: temp.path(),
            },
            deadline(),
        )
        .unwrap();
    let stage = std::cell::Cell::new("create");
    let result = (|| -> Result<()> {
        let job = Job::create(deadline())?;
        assert!(job.members(deadline())?.is_empty());
        let id = job.id().clone();
        // Test child cooperatively waits before doing anything. This is NOT an
        // atomic spawn containment implementation and is never used by product startup.
        stage.set("assign");
        job.assign_test_child(child.process())?;
        stage.set("observe");
        let process = Pinned::observe(child.process().identity().process_id(), deadline())?;
        stage.set("contains");
        assert!(job.contains(&process, deadline())?);
        assert_eq!(
            job.members(deadline())?,
            vec![process.identity().process_id()]
        );
        stage.set("other");
        let other = Job::create(deadline())?;
        assert!(!other.contains(&process, deadline())?);
        assert!(Job::create_named(id.clone(), deadline()).is_err());
        stage.set("reopen");
        let reopened = Job::reopen(&id, deadline())?;
        assert!(reopened.contains(&process, deadline())?);
        // Reopened handle has QUERY only: even assigning an already-member child is denied.
        assert!(reopened.assign_test_child(child.process()).is_err());
        drop(other);
        drop(job);
        process.verify(deadline())?;
        assert!(Job::reopen(&id, deadline())?.contains(&process, deadline())?);
        drop(reopened);
        process.verify(deadline())?;
        // Native observation: process membership keeps the object alive but
        // does not retain its namespace entry after the last handle closes.
        assert!(Job::reopen(&id, deadline()).is_err());
        std::fs::write(temp.path().join("release"), b"original release").unwrap();
        let until = deadline();
        loop {
            if child.process().verify(deadline()).is_err() {
                break;
            }
            assert!(Instant::now() < until, "original child timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(Job::reopen(&id, deadline()).is_err());
        Ok(())
    })();
    drop(release);
    assert!(
        result.is_ok(),
        "original Job stage {}: {:?}",
        stage.get(),
        result.err()
    );
}

#[test]
fn empty_job_disappears_and_invalid_or_expired_ids_do_not_open_objects() {
    let job = Job::create(deadline()).unwrap();
    let id = job.id().clone();
    assert!(
        Job::reopen(&id, deadline())
            .unwrap()
            .members(deadline())
            .unwrap()
            .is_empty()
    );
    drop(job);
    assert!(Job::reopen(&id, deadline()).is_err());
    for name in [
        "",
        "Global\\foreign",
        "Local\\qq-cli-123",
        "Local\\qq-cli-../../foreign",
    ] {
        let id = JobId(name.into());
        assert!(Job::reopen(&id, deadline()).is_err());
    }
    assert_eq!(Job::create(Instant::now()).err().unwrap().code, "DEADLINE");
    assert_eq!(
        Job::reopen(&JobId("invalid".into()), Instant::now())
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
}

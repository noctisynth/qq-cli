use super::*;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;

const FIXTURE: &str = "native::macos_copy_worker::tests::worker_fixture";
pub(crate) fn fixture(mode: &str, budget: u64, deadline: Instant) -> Worker {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", FIXTURE, "--nocapture"])
        .env_clear()
        .env("QQ_CLI_COPY_TEST", mode)
        .env("QQ_CLI_COPY_TEST_MS", budget.to_string());
    Worker::spawn(command, deadline).unwrap()
}

#[test]
fn worker_fixture() {
    let Ok(mode) = std::env::var("QQ_CLI_COPY_TEST") else {
        return;
    };
    let duration = Duration::from_millis(
        std::env::var("QQ_CLI_COPY_TEST_MS")
            .unwrap()
            .parse()
            .unwrap(),
    );
    let stream = UnixStream::from(rustix::io::dup(std::io::stdin().as_fd()).unwrap());
    match mode.as_str() {
        "live" => std::process::exit(i32::from(run(stream, duration))),
        "watch" => {
            watchdogs(&stream, duration).unwrap();
        }
        "ignore" => (),
        "unknown" => std::process::exit(0),
        _ => panic!("invalid synthetic mode"),
    }
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn files(root: &std::path::Path, index: usize) -> (File, File) {
    let source = root.join(format!("source-{index}"));
    std::fs::write(&source, b"original held content").unwrap();
    let destination = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(root.join(format!("destination-{index}")))
        .unwrap();
    (File::open(source).unwrap(), destination)
}

#[test]
fn persistent_worker_copies_held_files_and_finishes_with_confirmed_exit() {
    let temp = tempfile::tempdir().unwrap();
    let mut worker = fixture("live", 30000, Instant::now() + Duration::from_secs(15));
    for index in 0..20 {
        let (source, destination) = files(temp.path(), index);
        // Rename the pathname and replace it: the worker never receives names.
        std::fs::rename(
            temp.path().join(format!("source-{index}")),
            temp.path().join(format!("old-{index}")),
        )
        .unwrap();
        std::fs::write(temp.path().join(format!("source-{index}")), b"replacement").unwrap();
        worker
            .copy(source.as_fd(), destination.as_fd(), true, || Ok(()))
            .unwrap();
        assert_eq!(
            std::fs::read(temp.path().join(format!("destination-{index}"))).unwrap(),
            b"original held content"
        );
    }
    let pid = worker.child.id();
    let identity =
        crate::native::process::Pinned::observe(pid, Instant::now() + Duration::from_secs(5))
            .unwrap();
    worker.finish(|| Ok(())).unwrap();
    assert!(
        identity
            .verify(Instant::now() + Duration::from_secs(1))
            .is_err()
    );
}

#[test]
fn timeout_and_guard_failure_reap_worker_without_accepting_a_copy() {
    for guard_fails in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (source, destination) = files(temp.path(), 0);
        let mut worker = fixture("ignore", 30000, Instant::now() + Duration::from_millis(100));
        let identity = crate::native::process::Pinned::observe(
            worker.child.id(),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        let started = Instant::now();
        let calls = std::cell::Cell::new(0);
        let error = worker
            .copy(source.as_fd(), destination.as_fd(), true, || {
                calls.set(calls.get() + 1);
                if guard_fails && calls.get() >= 3 {
                    Err(AppError::storage())
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        if !guard_fails {
            assert_eq!(error.code, "DEADLINE");
        } else {
            assert!(calls.get() >= 3);
        }
        assert!(started.elapsed() < Duration::from_secs(3));
        assert!(worker.terminal);
        assert!(
            identity
                .verify(Instant::now() + Duration::from_secs(1))
                .is_err()
        );
        assert_eq!(destination.metadata().unwrap().len(), 0);
        assert!(
            worker
                .copy(source.as_fd(), destination.as_fd(), true, || Ok(()))
                .is_err()
        );
    }
}

#[test]
fn malformed_success_and_same_object_descriptors_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let (source, destination) = files(temp.path(), 0);
    let mut worker = fixture("unknown", 30000, Instant::now() + Duration::from_secs(3));
    assert!(
        worker
            .copy(source.as_fd(), destination.as_fd(), true, || Ok(()))
            .is_err()
    );
    assert_eq!(destination.metadata().unwrap().len(), 0);
    let mut worker = fixture("live", 30000, Instant::now() + Duration::from_secs(3));
    assert!(
        worker
            .copy(source.as_fd(), source.as_fd(), true, || Ok(()))
            .is_err()
    );
    assert_eq!(
        std::fs::read(temp.path().join("source-0")).unwrap(),
        b"original held content"
    );
}

#[test]
fn independent_watchdogs_exit_on_timeout_or_parent_socket_loss() {
    for close in [true, false] {
        let mut worker = fixture(
            "watch",
            if close { 30000 } else { 40 },
            Instant::now() + Duration::from_secs(5),
        );
        if close {
            worker.stream.shutdown(std::net::Shutdown::Both).unwrap();
        }
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = worker.child.try_wait().unwrap() {
                worker.terminal = true;
                assert_eq!(
                    status.code(),
                    Some(i32::from(if close { FAILURE } else { TIMEOUT }))
                );
                break;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn worker_rejects_incomplete_descriptor_requests_before_writing() {
    let temp = tempfile::tempdir().unwrap();
    let (source, destination) = files(temp.path(), 0);
    let mut worker = fixture("live", 30000, Instant::now() + Duration::from_secs(5));
    wire::send(&worker.stream, 2, &[source.as_fd()]).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = worker.child.try_wait().unwrap() {
            worker.terminal = true;
            assert_eq!(status.code(), Some(i32::from(FAILURE)));
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(destination.metadata().unwrap().len(), 0);
}

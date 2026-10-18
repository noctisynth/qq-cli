use std::io::{self, Write};
use std::sync::{Arc, atomic::AtomicBool, mpsc};
use std::time::{Duration, Instant};

use qq_cli::cli::stream_output::Output;
use qq_cli::messaging::watch::Sink;
use serde_json::json;

#[test]
fn bounded_writer_returns_on_deadline_and_never_reuses_a_blocked_worker() {
    struct Blocked {
        release: mpsc::Receiver<()>,
        finished: mpsc::SyncSender<()>,
    }
    impl Write for Blocked {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.release.recv().unwrap();
            self.finished.send(()).unwrap();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (release, wait) = mpsc::sync_channel(1);
    let (done, finished) = mpsc::sync_channel(1);
    let mut output = Output::new(
        Blocked {
            release: wait,
            finished: done,
        },
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let start = Instant::now();
    assert_eq!(
        output
            .record(
                &json!({"type":"message"}),
                start + Duration::from_millis(150)
            )
            .unwrap_err()
            .code,
        "OUTPUT_DEADLINE"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(output.started());
    assert_eq!(
        output
            .record(
                &json!({"type":"summary"}),
                Instant::now() + Duration::from_secs(1)
            )
            .unwrap_err()
            .code,
        "OUTPUT_IO"
    );
    release.send(()).unwrap();
    finished.recv_timeout(Duration::from_secs(2)).unwrap();
}

#[test]
fn worker_failure_and_cancellation_do_not_produce_extra_records() {
    struct Broken;
    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output::new(Broken, Arc::new(AtomicBool::new(false))).unwrap();
    assert_eq!(
        output
            .record(
                &json!({"type":"message"}),
                Instant::now() + Duration::from_secs(1)
            )
            .unwrap_err()
            .code,
        "OUTPUT_IO"
    );
    let mut output = Output::new(Vec::<u8>::new(), Arc::new(AtomicBool::new(true))).unwrap();
    assert_eq!(
        output
            .record(
                &json!({"type":"message"}),
                Instant::now() + Duration::from_secs(1)
            )
            .unwrap_err()
            .code,
        "CANCELLED"
    );
    assert!(!output.started());
}

#[test]
fn output_worker_process_exits_when_os_pipe_is_not_drained() {
    if std::env::var_os("QQ_CLI_TEST_BLOCKED_OUTPUT").is_some() {
        let mut output = Output::new(io::stdout(), Arc::new(AtomicBool::new(false))).unwrap();
        let error = output
            .record(
                &json!({"type":"message","body":"x".repeat(1024*1024)}),
                Instant::now() + Duration::from_millis(300),
            )
            .unwrap_err();
        std::process::exit(if error.code == "OUTPUT_DEADLINE" {
            0
        } else {
            1
        });
    }
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "output_worker_process_exits_when_os_pipe_is_not_drained",
            "--nocapture",
        ])
        .env("QQ_CLI_TEST_BLOCKED_OUTPUT", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() >= until {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("Blocked stdout prevented process exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

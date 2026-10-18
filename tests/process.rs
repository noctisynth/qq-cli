#![cfg(windows)]
use qq_cli::native::process::{Identity, Pinned};
use std::time::{Duration, Instant};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

#[test]
fn original_child_identity_is_pinned_and_exit_code_259_is_not_alive() {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    const CHILD: &str = "QQ_CLI_TEST_PINNED_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let mut byte = [0];
        std::io::stdin().read_exact(&mut byte).unwrap();
        std::process::exit(259);
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "original_child_identity_is_pinned_and_exit_code_259_is_not_alive",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // The test owns this child handle; no system process enumeration or QQ probe.
    let result = (|| -> qq_cli::core::error::Result<()> {
        let process = Pinned::observe(child.id(), deadline())?;
        let identity = process.identity().clone();
        assert_eq!(identity.process_id(), child.id());
        assert!(identity.created() > 0);
        assert_eq!(
            identity.image().canonicalize().unwrap(),
            std::env::current_exe().unwrap().canonicalize().unwrap()
        );
        process.verify(deadline())?;
        Pinned::reopen(&identity, deadline())?.verify(deadline())?;
        let mut changed = serde_json::to_value(&identity).unwrap();
        changed["created"] = serde_json::json!(identity.created() + 1);
        let changed: Identity = serde_json::from_value(changed).unwrap();
        assert_eq!(
            Pinned::reopen(&changed, deadline()).err().unwrap().code,
            "PROCESS_IDENTITY_CHANGED"
        );
        let mut changed = serde_json::to_value(&identity).unwrap();
        changed["image"] =
            serde_json::json!(identity.image().with_extension("synthetic-unrelated"));
        let changed: Identity = serde_json::from_value(changed).unwrap();
        assert_eq!(
            Pinned::reopen(&changed, deadline()).err().unwrap().code,
            "PROCESS_IDENTITY_CHANGED"
        );
        child.stdin.take().unwrap().write_all(b"x").unwrap();
        let until = deadline();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(259));
                break;
            }
            assert!(Instant::now() < until, "original child did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(
            process.verify(deadline()).unwrap_err().code,
            "PROCESS_EXITED"
        );
        Ok(())
    })();
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
        child.wait().unwrap();
    }
    result.unwrap();
}

#[test]
fn invalid_and_expired_observations_are_rejected_without_native_probes() {
    assert_eq!(
        Pinned::observe(0, deadline()).err().unwrap().code,
        "INVALID_ARGUMENTS"
    );
    assert_eq!(
        Pinned::observe(u32::MAX, Instant::now())
            .err()
            .unwrap()
            .code,
        "DEADLINE"
    );
    let invalid: Identity = serde_json::from_value(
        serde_json::json!({"processId":0,"created":0,"image":"relative-fixture"}),
    )
    .unwrap();
    assert_eq!(
        Pinned::reopen(&invalid, deadline()).err().unwrap().code,
        "PROCESS_IDENTITY_CHANGED"
    );
}

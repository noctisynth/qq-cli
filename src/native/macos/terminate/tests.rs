use super::*;
use std::io::{BufRead, Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}
struct Cleanup(Child);
impl Drop for Cleanup {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
fn child(mode: &str) -> Cleanup {
    Cleanup(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::macos_terminate::tests::synthetic_target",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("QQ_CLI_TERMINATE_FIXTURE", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
#[test]
fn synthetic_target() {
    let Ok(mode) = std::env::var("QQ_CLI_TERMINATE_FIXTURE") else {
        return;
    };
    std::io::stdin().read_exact(&mut [0]).unwrap();
    if mode == "exec" {
        use std::os::unix::process::CommandExt;
        panic!(
            "synthetic exec failed: {}",
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native::macos_terminate::tests::synthetic_replacement",
                    "--nocapture",
                    "--test-threads=1"
                ])
                .exec()
        );
    }
}
#[test]
fn synthetic_replacement() {
    if std::env::var_os("QQ_CLI_TERMINATE_FIXTURE").is_none() {
        return;
    }
    println!("replacement-ready");
    std::io::stdout().flush().unwrap();
    let _ = std::io::stdin().read_exact(&mut [0]);
}
#[test]
fn exact_target_roundtrips_and_dispatch_does_not_claim_exit_until_observed() {
    let mut child = child("wait");
    let owner = Pinned::observe(child.0.id(), deadline()).unwrap();
    let target = Target::capture(&owner, deadline()).unwrap();
    let bytes = serde_json::to_vec(&target).unwrap();
    let restored: Target = serde_json::from_slice(&bytes).unwrap();
    assert!(restored.identity() == owner.identity());
    restored.reopen(deadline()).unwrap();
    assert!(child.0.try_wait().unwrap().is_none());
    restored.terminate_once(&owner, deadline()).unwrap();
    let until = deadline();
    while child.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(owner.verify(deadline()).unwrap_err().code, "PROCESS_EXITED");
    // A repeated attempt fails in the original observation, before another signal call.
    assert_eq!(
        restored
            .terminate_once(&owner, deadline())
            .unwrap_err()
            .code,
        "PROCESS_EXITED"
    );
}
#[test]
fn corrupted_foreign_and_expired_targets_never_signal_either_child() {
    let mut first = child("wait");
    let mut second = child("wait");
    let owner = Pinned::observe(first.0.id(), deadline()).unwrap();
    let other = Pinned::observe(second.0.id(), deadline()).unwrap();
    let target = Target::capture(&owner, deadline()).unwrap();
    assert_eq!(
        target.terminate_once(&other, deadline()).unwrap_err().code,
        "PROCESS_TERMINATION_CONFLICT"
    );
    assert_eq!(
        target
            .terminate_once(&owner, Instant::now())
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    let mut corrupt = target.clone();
    corrupt.audit_token[0] ^= 1;
    assert_eq!(
        corrupt.reopen(deadline()).err().unwrap().code,
        "PROCESS_TERMINATION_CONFLICT"
    );
    assert_eq!(
        corrupt.terminate_once(&owner, deadline()).unwrap_err().code,
        "PROCESS_TERMINATION_CONFLICT"
    );
    let mut corrupt = target.clone();
    corrupt.schema_version = 2;
    assert_eq!(
        corrupt.reopen(deadline()).err().unwrap().code,
        "PROCESS_TERMINATION_CONFLICT"
    );
    assert!(first.0.try_wait().unwrap().is_none());
    assert!(second.0.try_wait().unwrap().is_none());
    for dispatch in [Target::suspend_once, Target::resume_once] {
        assert_eq!(
            dispatch(&target, &other, deadline()).unwrap_err().code,
            "PROCESS_TERMINATION_CONFLICT"
        );
        assert_eq!(
            dispatch(&target, &owner, Instant::now()).unwrap_err().code,
            "DEADLINE"
        );
        assert_eq!(
            dispatch(&corrupt, &owner, deadline()).unwrap_err().code,
            "PROCESS_TERMINATION_CONFLICT"
        );
    }
    assert!(!owner.is_suspended(deadline()).unwrap());
    assert!(!other.is_suspended(deadline()).unwrap());
}
#[test]
fn kernel_refuses_original_generation_after_same_pid_exec() {
    let mut child = child("exec");
    let owner = Pinned::observe(child.0.id(), deadline()).unwrap();
    let target = Target::capture(&owner, deadline()).unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            if line.unwrap().ends_with("replacement-ready") {
                let _ = send.send(());
                return;
            }
        }
    });
    child.0.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    receive.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        target.terminate_once(&owner, deadline()).unwrap_err().code,
        "PROCESS_IDENTITY_CHANGED"
    );
    // SAFETY: this test deliberately exercises kernel rejection using the original token of its own
    // still-owned unreaped child. The buffer is exactly eight initialized SDK words.
    let refused = unsafe { qq_cli_terminate_audit_token(target.audit_token.as_ptr()) };
    assert_ne!(refused, 0);
    assert!(child.0.try_wait().unwrap().is_none());
    let current = Pinned::observe(child.0.id(), deadline()).unwrap();
    let current_target = Target::capture(&current, deadline()).unwrap();
    assert!(current_target.audit_token != target.audit_token);
    for dispatch in [Target::suspend_once, Target::resume_once] {
        assert_eq!(
            dispatch(&target, &owner, deadline()).unwrap_err().code,
            "PROCESS_IDENTITY_CHANGED"
        );
    }
    // SAFETY: original token of the still-owned unreaped synthetic child, as above.
    assert_ne!(
        unsafe { qq_cli_suspend_audit_token(target.audit_token.as_ptr()) },
        0
    );
    // SAFETY: same bounded original token; kernel must reject the stale exec generation.
    assert_ne!(
        unsafe { qq_cli_resume_audit_token(target.audit_token.as_ptr()) },
        0
    );
    assert!(!current.is_suspended(deadline()).unwrap());
}

#[test]
fn original_generation_suspends_and_reopens_before_explicit_resume() {
    let mut child = child("wait");
    let owner = Pinned::observe(child.0.id(), deadline()).unwrap();
    let target = Target::capture(&owner, deadline()).unwrap();
    assert!(!owner.is_suspended(deadline()).unwrap());
    target.suspend_once(&owner, deadline()).unwrap();
    let until = deadline();
    while !owner.is_suspended(until).unwrap() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    let serialized = serde_json::to_vec(&target).unwrap();
    let restored: Target = serde_json::from_slice(&serialized).unwrap();
    let recovered = restored.reopen(deadline()).unwrap();
    assert!(recovered.is_suspended(deadline()).unwrap());
    // Data is available, but a suspended process cannot consume it and exit.
    child.0.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert!(child.0.try_wait().unwrap().is_none());
    assert!(owner.is_suspended(deadline()).unwrap());
    restored.resume_once(&recovered, deadline()).unwrap();
    let until = deadline();
    while child.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        recovered.is_suspended(deadline()).unwrap_err().code,
        "PROCESS_EXITED"
    );
}

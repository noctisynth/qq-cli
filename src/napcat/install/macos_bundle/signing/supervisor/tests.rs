use super::*;
use std::io::{BufRead, BufReader, Write};

const FIXTURE: &str = "napcat::install::macos_bundle::signing::supervisor::tests::fixture";
fn command(mode: &str) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", FIXTURE, "--nocapture"])
        .env("QQ_CLI_SUPERVISOR_FIXTURE", mode);
    cmd
}
#[test]
fn fixture() {
    let Ok(mode) = std::env::var("QQ_CLI_SUPERVISOR_FIXTURE") else {
        return;
    };
    if mode == "writer" {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    if mode == "unexpected" {
        std::process::exit(0);
    }
    if mode == "owner" {
        let mut supervisor = command("eof")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut output = BufReader::new(supervisor.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert!(output.read_line(&mut line).unwrap() > 0);
            if line.starts_with("WRITER:") {
                println!("SUPERVISOR:{}", supervisor.id());
                print!("{line}");
                std::io::stdout().flush().unwrap();
                break;
            }
        }
        // The test kills this owner without running destructors. Its pipe is
        // the only parent-liveness writer; the signing child cannot inherit it.
        loop {
            if supervisor.try_wait().unwrap().is_some() {
                std::process::exit(FAILED);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    let alive = watch_parent().unwrap();
    let child = command("writer")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    println!("WRITER:{}", child.id());
    std::io::stdout().flush().unwrap();
    let deadline = Instant::now() + Duration::from_secs(if mode == "deadline" { 1 } else { 10 });
    let result = execute::wait(child, deadline, &|| {
        if alive.load(Ordering::Acquire) {
            Ok(())
        } else {
            Err(invalid())
        }
    });
    std::process::exit(match result {
        Ok(()) => SIGNED,
        Err(e) if e.code == "SIGNING_CLEANUP_UNKNOWN" => UNKNOWN,
        Err(e) if e.code == "DEADLINE" => EXPIRED,
        Err(_) => FAILED,
    });
}
fn start(mode: &str) -> (Child, crate::native::process::Pinned) {
    let mut child = command(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    let pid = loop {
        line.clear();
        assert!(output.read_line(&mut line).unwrap() > 0);
        if let Some(pid) = line.trim().strip_prefix("WRITER:") {
            break pid.parse().unwrap();
        }
    };
    let observed =
        crate::native::process::Pinned::observe(pid, Instant::now() + Duration::from_secs(5))
            .unwrap();
    (child, observed)
}
#[test]
fn parent_eof_guard_loss_and_worker_deadline_reap_the_writer() {
    for mode in ["eof", "guard", "deadline"] {
        let (mut child, writer) = start(mode);
        if mode == "eof" {
            child.stdin.take();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let result = await_supervisor(child, deadline, &|| {
            if mode == "guard" {
                Err(AppError::new("GUARD_LOST", "Synthetic guard loss", 3))
            } else {
                Ok(())
            }
        });
        assert_eq!(
            result.unwrap_err().code,
            match mode {
                "guard" => "GUARD_LOST",
                "deadline" => "DEADLINE",
                _ => "MACOS_BUNDLE_CHANGED",
            }
        );
        assert!(writer.verify(deadline).is_err());
    }
}
#[test]
fn unrecognized_supervisor_exit_is_never_a_cleanup_receipt() {
    let child = command("unexpected")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert_eq!(
        await_supervisor(child, Instant::now() + Duration::from_secs(5), &|| Ok(()))
            .unwrap_err()
            .code,
        "SIGNING_CLEANUP_UNKNOWN"
    );
}

#[test]
fn abrupt_owner_death_leaves_a_live_supervisor_to_reap_its_writer() {
    let mut owner = command("owner")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(owner.stdout.take().unwrap());
    let mut pids = Vec::new();
    while pids.len() < 2 {
        let mut line = String::new();
        assert!(output.read_line(&mut line).unwrap() > 0);
        if let Some(pid) = line
            .trim()
            .strip_prefix("SUPERVISOR:")
            .or_else(|| line.trim().strip_prefix("WRITER:"))
        {
            pids.push(pid.parse::<u32>().unwrap());
        }
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    let observed: Vec<_> = pids
        .into_iter()
        .map(|pid| crate::native::process::Pinned::observe(pid, deadline).unwrap())
        .collect();
    assert!(owner.try_wait().unwrap().is_none());
    owner.kill().unwrap();
    owner.wait().unwrap();
    while observed
        .iter()
        .any(|p| p.verify(Instant::now() + Duration::from_secs(1)).is_ok())
    {
        assert!(
            Instant::now() < deadline,
            "orphan signing process remained live"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

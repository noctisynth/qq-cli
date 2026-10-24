use super::*;
use std::time::Duration;
const FIXTURE: &str = "napcat::install::macos::source::image::attach::worker::tests::fixture";
fn command(mode: &str, base: &Path) -> Command {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", FIXTURE, "--nocapture"])
        .env("QQ_CLI_ATTACH_FIXTURE", mode)
        .env("QQ_CLI_ATTACH_BASE", base)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}
#[test]
fn fixture() {
    let Ok(mode) = std::env::var("QQ_CLI_ATTACH_FIXTURE") else {
        return;
    };
    let base = PathBuf::from(std::env::var_os("QQ_CLI_ATTACH_BASE").unwrap());
    if mode == "command" {
        std::fs::write(base.join("command"), std::process::id().to_string()).unwrap();
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    if mode == "unexpected" {
        std::process::exit(0);
    }
    if mode == "owner" {
        let mut child = command("supervisor", &base).spawn().unwrap();
        child.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
        std::fs::write(base.join("supervisor"), child.id().to_string()).unwrap();
        loop {
            if child.try_wait().unwrap().is_some() {
                std::process::exit(FAILED);
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    let deadline = Instant::now() + Duration::from_secs(if mode == "deadline" { 2 } else { 20 });
    let parent = Parent::prepare(deadline).unwrap();
    let executable = std::env::current_exe().unwrap();
    // The fixture writes its PID after exec so process-generation checks cannot
    // mistake a shell-to-command exec for the command having exited.
    let result = mounted::command::run(
        "/usr/bin/env",
        &[
            OsStr::new("QQ_CLI_ATTACH_FIXTURE=command"),
            std::ffi::OsString::from(format!("QQ_CLI_ATTACH_BASE={}", base.display())).as_os_str(),
            executable.as_os_str(),
            OsStr::new("--exact"),
            OsStr::new(FIXTURE),
            OsStr::new("--nocapture"),
        ],
        None,
        deadline,
        &|| parent.check(deadline),
    );
    std::process::exit(match result {
        Ok(_) => FINISHED,
        Err(e) if e.code == "MOUNT_QUERY_EXIT_UNKNOWN" => UNKNOWN,
        Err(e) if e.code == "DEADLINE" => EXPIRED,
        Err(_) => FAILED,
    });
}
fn observed(path: &Path) -> crate::native::process::Pinned {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
            && let Ok(pid) = text.parse::<u32>()
        {
            return crate::native::process::Pinned::observe(pid, deadline).unwrap();
        }
        assert!(Instant::now() < deadline, "fixture did not start");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn eof_guard_and_deadline_reap_the_direct_command() {
    for mode in ["eof", "guard", "deadline"] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let mut child = command(mode, &base).spawn().unwrap();
        child.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
        let process = observed(&base.join("command"));
        if mode == "eof" {
            child.stdin.take();
        }
        let mut supervisor = Supervisor(Some(child));
        let deadline = Instant::now() + Duration::from_secs(5);
        let error = supervisor
            .wait(deadline, &|| {
                if mode == "guard" {
                    Err(AppError::new("GUARD_LOST", "Synthetic guard loss", 3))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(
            error.code,
            match mode {
                "guard" => "GUARD_LOST",
                "deadline" => "DEADLINE",
                _ => "QQ_MOUNT_EFFECT_FAILED",
            }
        );
        assert!(process.verify(deadline).is_err());
    }
}
#[test]
fn abrupt_owner_death_keeps_supervisor_alive_to_reap_its_command() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let mut owner = command("owner", &base).spawn().unwrap();
    let supervisor = observed(&base.join("supervisor"));
    let child = observed(&base.join("command"));
    owner.kill().unwrap();
    owner.wait().unwrap();
    let end = Instant::now() + Duration::from_secs(5);
    while supervisor
        .verify(Instant::now() + Duration::from_secs(1))
        .is_ok()
        || child
            .verify(Instant::now() + Duration::from_secs(1))
            .is_ok()
    {
        assert!(
            Instant::now() < end,
            "original fixture process still running"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn unexpected_supervisor_exit_does_not_prove_command_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let child = command("unexpected", temp.path()).spawn().unwrap();
    let mut supervisor = Supervisor(Some(child));
    assert_eq!(
        supervisor
            .wait(Instant::now() + Duration::from_secs(5), &|| Ok(()))
            .unwrap_err()
            .code,
        "QQ_MOUNT_EFFECT_EXIT_UNKNOWN"
    );
}

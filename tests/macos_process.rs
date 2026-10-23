#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::native::process::{Identity, Pinned};
use std::io::{BufRead, Read, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
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
fn child(name: &str, mode: &str) -> Cleanup {
    Cleanup(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--test-threads=1", "--nocapture"])
            .env("QQ_CLI_SYNTHETIC_PROCESS", mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn run_child() {
    let Some(mode) = std::env::var_os("QQ_CLI_SYNTHETIC_PROCESS") else {
        return;
    };
    if mode == "detach" {
        use std::os::unix::process::CommandExt;
        let child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "original_child_identity_reopens_and_exit_remains_terminal",
                "--test-threads=1",
            ])
            .env("QQ_CLI_SYNTHETIC_PROCESS", "exit")
            .process_group(0)
            .stdin(Stdio::inherit())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        println!("orphan-pid={}", child.id());
        std::io::stdout().flush().unwrap();
        std::process::exit(0);
    }
    let mut byte = [0];
    std::io::stdin().read_exact(&mut byte).unwrap();
    if mode == "exec" {
        use std::os::unix::process::CommandExt;
        println!("exec-ready");
        std::io::stdout().flush().unwrap();
        let error = Command::new("/bin/cat").exec();
        panic!("synthetic exec failed: {error}");
    }
    std::process::exit(0);
}
#[test]
fn original_child_identity_reopens_and_exit_remains_terminal() {
    run_child();
    let mut child = child(
        "original_child_identity_reopens_and_exit_remains_terminal",
        "exit",
    );
    let pinned = Pinned::observe(child.0.id(), deadline()).unwrap();
    let identity = pinned.identity().clone();
    assert_eq!(identity.process_id(), child.0.id());
    assert_eq!(
        identity.image().canonicalize().unwrap(),
        std::env::current_exe().unwrap().canonicalize().unwrap()
    );
    let wire = serde_json::to_value(&identity).unwrap();
    assert_eq!(wire["platform"], "macos");
    assert!(wire.get("created").is_none());
    Pinned::reopen(&identity, deadline())
        .unwrap()
        .verify(deadline())
        .unwrap();
    for (name, value) in [
        (
            "startMicroseconds",
            serde_json::json!((wire["startMicroseconds"].as_u64().unwrap() + 1) % 1000000),
        ),
        (
            "bootSession",
            serde_json::json!("00000000-0000-0000-0000-000000000001"),
        ),
        ("image", serde_json::json!("/synthetic/another-process")),
    ] {
        let mut changed = wire.clone();
        changed[name] = value;
        let changed: Identity = serde_json::from_value(changed).unwrap();
        assert_eq!(
            Pinned::reopen(&changed, deadline()).err().unwrap().code,
            "PROCESS_IDENTITY_CHANGED"
        );
    }
    child.0.stdin.take().unwrap().write_all(b"x").unwrap();
    let until = deadline();
    while child.0.try_wait().unwrap().is_none() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    for _ in 0..2 {
        assert_eq!(
            pinned.verify(deadline()).unwrap_err().code,
            "PROCESS_EXITED"
        );
    }
}
#[test]
fn exec_invalidates_the_original_observation_even_while_pid_remains_alive() {
    run_child();
    let mut child = child(
        "exec_invalidates_the_original_observation_even_while_pid_remains_alive",
        "exec",
    );
    let pinned = Pinned::observe(child.0.id(), deadline()).unwrap();
    child.0.stdin.as_mut().unwrap().write_all(b"x").unwrap();
    let stdout = child.0.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            if line.ends_with("exec-ready\n") {
                let _ = send.send(());
                return;
            }
        }
    });
    receive.recv_timeout(Duration::from_secs(5)).unwrap();
    let until = deadline();
    loop {
        match pinned.verify(until) {
            Ok(()) => {
                assert!(Instant::now() < until);
                std::thread::yield_now();
            }
            Err(error) => {
                assert_eq!(error.code, "PROCESS_IDENTITY_CHANGED");
                break;
            }
        }
    }
    assert!(child.0.try_wait().unwrap().is_none());
    for _ in 0..2 {
        assert_eq!(
            pinned.verify(deadline()).unwrap_err().code,
            "PROCESS_IDENTITY_CHANGED"
        );
    }
}
#[test]
fn invalid_foreign_and_expired_requests_do_not_probe_processes() {
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
    let foreign: Identity = serde_json::from_value(
        serde_json::json!({"processId":std::process::id(),"created":1,"image":"C:\\synthetic.exe"}),
    )
    .unwrap();
    assert_eq!(
        Pinned::reopen(&foreign, deadline()).err().unwrap().code,
        "PROCESS_PLATFORM_MISMATCH"
    );
}

#[test]
fn exact_executable_discovery_filters_readers_and_rejects_file_replacement() {
    use qq_cli::native::process::Executable;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap().join("owned-process");
    std::fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    let executable = Executable::open(&path, deadline()).unwrap();
    // This test process holds an open file reference, but must never become a candidate.
    let _reader = std::fs::File::open(&path).unwrap();
    assert!(executable.candidates(deadline()).unwrap().is_empty());
    let mut child = Cleanup(
        Command::new(&path)
            .args([
                "--exact",
                "original_child_identity_reopens_and_exit_remains_terminal",
                "--test-threads=1",
            ])
            .env("QQ_CLI_SYNTHETIC_PROCESS", "exit")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let candidates = executable.candidates(deadline()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].identity().process_id(), child.0.id());
    child.0.stdin.take().unwrap().write_all(b"x").unwrap();
    child.0.wait().unwrap();
    assert!(executable.candidates(deadline()).unwrap().is_empty());
    let mut launcher = Cleanup(
        Command::new(&path)
            .args([
                "--exact",
                "original_child_identity_reopens_and_exit_remains_terminal",
                "--test-threads=1",
                "--nocapture",
            ])
            .env("QQ_CLI_SYNTHETIC_PROCESS", "detach")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut output = String::new();
    launcher
        .0
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    // Child::wait closes stdin; retain our pipe until the orphan is observed and released.
    let mut input = launcher.0.stdin.take().unwrap();
    assert!(launcher.0.wait().unwrap().success());
    let orphan: u32 = output
        .split("orphan-pid=")
        .nth(1)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let candidates = executable.candidates(deadline()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].identity().process_id(), orphan);
    input.write_all(b"x").unwrap();
    drop(input);
    let until = deadline();
    while candidates[0].verify(until).is_ok() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::rename(&path, path.with_extension("old")).unwrap();
    std::fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    assert_eq!(
        executable.candidates(deadline()).err().unwrap().code,
        "PROCESS_CENSUS_UNVERIFIABLE"
    );
}

#[test]
fn executable_discovery_rejects_links_permissions_and_expired_deadlines() {
    use qq_cli::native::process::Executable;
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().canonicalize().unwrap().join("owned-process");
    std::fs::copy(std::env::current_exe().unwrap(), &path).unwrap();
    let executable = Executable::open(&path, deadline()).unwrap();
    assert_eq!(
        executable.candidates(Instant::now()).err().unwrap().code,
        "DEADLINE"
    );
    let alias = path.with_extension("alias");
    symlink(&path, &alias).unwrap();
    assert!(Executable::open(&alias, deadline()).is_err());
    std::fs::remove_file(&alias).unwrap();
    std::fs::hard_link(&path, &alias).unwrap();
    assert!(Executable::open(&path, deadline()).is_err());
    assert!(executable.verify(deadline()).is_err());
    std::fs::remove_file(&alias).unwrap();
    let executable = Executable::open(&path, deadline()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert!(executable.verify(deadline()).is_err());
    assert!(Executable::open(&path, deadline()).is_err());
}

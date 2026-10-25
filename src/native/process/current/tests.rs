use super::*;
use std::os::unix::fs::{MetadataExt, symlink};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

fn limit() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

fn wait(child: &mut Child, deadline: Instant) -> ExitStatus {
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("synthetic image fixture timed out");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn image_fixture() {
    let Some(root) = std::env::var_os("QQ_CLI_SYNTHETIC_SELF_IMAGE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let original = PathBuf::from(std::env::var_os("QQ_CLI_SYNTHETIC_SELF_IMAGE_ORIGINAL").unwrap());
    std::fs::write(root.join("ready"), b"ready").unwrap();
    let deadline = limit();
    while !root.join("continue").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    let resolved = executable(deadline).unwrap();
    assert!(!resolved.is_symlink());
    let observed = std::fs::metadata(resolved).unwrap();
    let expected = std::fs::metadata(original).unwrap();
    assert_eq!(
        (observed.dev(), observed.ino()),
        (expected.dev(), expected.ino())
    );
}

#[test]
fn switching_the_launcher_does_not_redirect_the_running_image() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let original = std::env::current_exe().unwrap().canonicalize().unwrap();
    let launcher = root.join("qq-cli");
    symlink(&original, &launcher).unwrap();
    let mut child = Command::new(&launcher)
        .args([
            "--exact",
            "native::process::current::tests::image_fixture",
            "--test-threads=1",
        ])
        .env("QQ_CLI_SYNTHETIC_SELF_IMAGE_ROOT", &root)
        .env("QQ_CLI_SYNTHETIC_SELF_IMAGE_ORIGINAL", &original)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = limit();
    while !root.join("ready").exists() {
        assert!(
            child.try_wait().unwrap().is_none(),
            "fixture exited before ready"
        );
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("synthetic image fixture did not start");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // Match the installer's atomic command-link switch after the original
    // executable is loaded. Following current_exe's launch link is now wrong.
    let replacement = root.join("other-program");
    std::fs::write(&replacement, b"must never be selected as the self worker").unwrap();
    let next = root.join("next");
    symlink(&replacement, &next).unwrap();
    std::fs::rename(next, &launcher).unwrap();
    std::fs::write(root.join("continue"), b"continue").unwrap();
    assert!(wait(&mut child, deadline).success());
    assert_eq!(std::fs::read_link(launcher).unwrap(), replacement);
}

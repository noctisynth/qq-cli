#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::native::macos_signature::verify_original;
use std::path::Path;
use std::time::{Duration, Instant};

#[test]
fn invalid_paths_and_expired_deadlines_never_establish_publisher_identity() {
    assert!(
        verify_original(
            Path::new("relative.app"),
            Instant::now() + Duration::from_secs(5)
        )
        .is_err()
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let app = root.join("Empty.app");
    std::fs::create_dir(&app).unwrap();
    assert!(verify_original(&app, Instant::now()).is_err());
    assert!(verify_original(&app, Instant::now() + Duration::from_secs(5)).is_err());
    let alias = root.join("Alias.app");
    std::os::unix::fs::symlink(&app, &alias).unwrap();
    assert!(verify_original(&alias, Instant::now() + Duration::from_secs(5)).is_err());
}

#[test]
fn valid_adhoc_signature_with_qq_identifier_is_not_an_official_source() {
    // Only this test binary is copied/signed; no upstream or user application.
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("Synthetic.app");
    let contents = root.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    std::fs::create_dir(contents.join("Resources")).unwrap();
    std::fs::copy(
        std::env::current_exe().unwrap(),
        contents.join("MacOS/Synthetic"),
    )
    .unwrap();
    std::fs::write(contents.join("Info.plist"),r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.tencent.qq</string><key>CFBundleExecutable</key><string>Synthetic</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleVersion</key><string>1</string></dict></plist>"#).unwrap();
    for args in [
        vec!["--force", "--sign", "-", "--identifier", "com.tencent.qq"],
        vec!["--verify", "--deep", "--strict"],
    ] {
        let output = std::process::Command::new("/usr/bin/codesign")
            .args(args)
            .arg(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let result = verify_original(&root, Instant::now() + Duration::from_secs(15));
    assert_eq!(result.unwrap_err().code, "QQ_SIGNATURE_UNVERIFIED");
    // Keep stdin open so this checks native rejection, not parent-EOF cancellation.
    let mut worker = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args([
            "--internal-macos-signature-v1",
            root.to_str().unwrap(),
            "10000",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let until = Instant::now() + Duration::from_secs(12);
    loop {
        if let Some(status) = worker.try_wait().unwrap() {
            assert_eq!(status.code(), Some(93));
            break;
        }
        if Instant::now() >= until {
            let _ = worker.kill();
            let _ = worker.wait();
            panic!("native rejection worker failed to finish");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn private_worker_protocol_is_silent_and_rejects_malformed_requests() {
    for args in [
        vec!["--internal-macos-signature-v1"],
        vec!["--internal-macos-signature-v1", "/missing.app", "0"],
        vec![
            "--internal-macos-signature-v1",
            "/missing.app",
            "1000",
            "extra",
        ],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(93));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn private_worker_closes_when_its_parent_liveness_pipe_closes() {
    use std::process::{Command, Stdio};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("Empty.app");
    std::fs::create_dir(&root).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .args([
            "--internal-macos-signature-v1",
            root.to_str().unwrap(),
            "300000",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // No payload or credential is transmitted. EOF cancels the read-only worker.
    drop(child.stdin.take());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert_eq!(status.code(), Some(93));
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("worker did not exit on parent pipe closure");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

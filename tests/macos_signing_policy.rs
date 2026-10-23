#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::napcat::install::macos_signing::Policy;
use std::path::Path;
use std::time::{Duration, Instant};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(15)
}

#[test]
fn actual_signatures_require_exact_roles_boolean_entitlements_and_runtime_flags() {
    for variant in 0..11 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let binary = base.join("Synthetic");
        // Original test code only; signed bytes are never executed.
        std::fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
        let policy = match variant {
            1 => Policy::Electron,
            2 | 10 => Policy::Native,
            _ => Policy::Main,
        };
        let mut xml = policy.entitlements();
        if variant == 3 {
            xml = Policy::Electron.entitlements();
        }
        if variant == 4 {
            xml = xml.replace(
                "</dict>",
                "<key>com.apple.security.app-sandbox</key><true/></dict>",
            );
        }
        if variant == 5 {
            xml = xml.replacen("<true/>", "<false/>", 1);
        }
        if variant == 6 {
            xml = xml.replacen("<true/>", "<string>true</string>", 1);
        }
        let entitlements = base.join("entitlements.plist");
        std::fs::write(&entitlements, xml).unwrap();
        let mut command = std::process::Command::new("/usr/bin/codesign");
        command.env_clear().args([
            "--force",
            "--sign",
            "-",
            "--timestamp=none",
            "--generate-entitlement-der",
        ]);
        if variant != 7 {
            command.args(["--options", "runtime"]);
        }
        if variant != 10 {
            command.arg("--entitlements").arg(&entitlements);
        }
        let result = command.arg(&binary).output().unwrap();
        assert!(
            result.status.success(),
            "synthetic signing failed in variant {variant}"
        );
        if variant == 8 {
            use std::io::{Seek, SeekFrom, Write};
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .open(&binary)
                .unwrap();
            file.seek(SeekFrom::Start(16384)).unwrap();
            file.write_all(b"synthetic tampering").unwrap();
        }
        let verify = if variant == 9 { Policy::Native } else { policy };
        let result = verify.verify(&binary, deadline());
        let role = match verify {
            Policy::Main => "main",
            Policy::Electron => "electron",
            Policy::Native => "native",
        };
        let mut worker = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .arg("--internal-macos-policy-v1")
            .arg(&binary)
            .args([role, "15000"])
            .env_clear()
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        // Hold the parent-liveness pipe until the worker has exited.
        let alive = worker.stdin.take().unwrap();
        let output = worker.wait_with_output().unwrap();
        drop(alive);
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
        assert_eq!(
            output.status.code(),
            Some(if variant < 3 || variant == 10 { 95 } else { 93 })
        );
        if variant < 3 || variant == 10 {
            result.unwrap();
        } else {
            assert!(result.is_err(), "variant {variant}");
        }
    }
}

#[test]
fn signing_observation_rejects_aliases_and_expired_or_unsigned_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let file = base.join("unsigned");
    std::fs::write(&file, b"synthetic unsigned data").unwrap();
    let alias = base.join("alias");
    std::os::unix::fs::symlink(&file, &alias).unwrap();
    assert!(Policy::Native.verify(&file, Instant::now()).is_err());
    assert!(Policy::Native.verify(&file, deadline()).is_err());
    assert!(Policy::Native.verify(&alias, deadline()).is_err());
    assert!(
        Policy::Native
            .verify(Path::new("relative"), deadline())
            .is_err()
    );
}

#[test]
fn policy_worker_rejects_malformed_roles_budgets_and_extra_arguments_silently() {
    for args in [
        vec![],
        vec!["/missing", "native", "0"],
        vec!["/missing", "native", "300001"],
        vec!["/missing", "unknown", "1000"],
        vec!["/missing", "main", "1000", "extra"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_qq-cli"))
            .arg("--internal-macos-policy-v1")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(93));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    let error = Policy::Native
        .verify_isolated(Path::new("/missing"), deadline(), || {
            Err(qq_cli::core::error::AppError::new(
                "GUARD_LOST",
                "Synthetic guard loss",
                3,
            ))
        })
        .unwrap_err();
    assert_eq!(error.code, "GUARD_LOST");
    assert_eq!(
        Policy::Native
            .verify_isolated(Path::new("/missing"), Instant::now(), || Ok(()))
            .unwrap_err()
            .code,
        "DEADLINE"
    );
}

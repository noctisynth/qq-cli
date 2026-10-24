#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::napcat::install::macos_signing::Policy;
use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn invoke(path: &Path, target: File, input: File, role: &str) -> i32 {
    let mut child = Command::new(env!("CARGO_BIN_EXE_qq-cli"))
        .arg("--internal-macos-signer-v1")
        .arg(path)
        .args([role, "10000"])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::from(input))
        .stderr(Stdio::from(target))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap();
        }
        assert!(Instant::now() < deadline, "supervisor failed to terminate");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn input(policy: Policy) -> File {
    let mut file = tempfile::tempfile().unwrap();
    file.write_all(policy.entitlements().as_bytes()).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file
}
fn binary(path: &Path) {
    // Original test executable; it is signed but never executed.
    std::fs::copy(std::env::current_exe().unwrap(), path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
#[test]
fn actual_supervisor_signs_all_fixed_roles_using_held_anonymous_inputs() {
    for (role, policy) in [
        ("main", Policy::Main),
        ("electron", Policy::Electron),
        ("plugin-host", Policy::PluginHost),
        ("native", Policy::Native),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().canonicalize().unwrap().join("Synthetic");
        binary(&path);
        assert_eq!(
            invoke(&path, File::open(&path).unwrap(), input(policy), role),
            109
        );
        policy
            .verify(&path, Instant::now() + Duration::from_secs(10))
            .unwrap();
    }
}
#[test]
fn wrong_target_descriptor_nonprivate_target_and_wrong_entitlements_do_not_sign() {
    for variant in 0..4 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let path = base.join("Synthetic");
        binary(&path);
        let original = std::fs::read(&path).unwrap();
        let other = base.join("Other");
        binary(&other);
        let target = File::open(if variant == 0 { &other } else { &path }).unwrap();
        if variant == 1 {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let entitlement = if variant == 3 {
            let named = base.join("linked.xml");
            std::fs::write(&named, Policy::Main.entitlements()).unwrap();
            File::open(named).unwrap()
        } else {
            input(if variant == 2 {
                Policy::Native
            } else {
                Policy::Main
            })
        };
        assert_eq!(invoke(&path, target, entitlement, "main"), 110);
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

use super::*;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
fn name() -> String {
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).unwrap();
    format!(
        "Local\\qq-cli-vault-test-{}",
        nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}
#[test]
fn gate_is_noninheritable_and_excludes_other_threads_until_drop() {
    let name = name();
    let held = acquire_named(&name, 100).unwrap();
    let mut flags = 0;
    // SAFETY: live privately held test handle; writable flags output.
    unsafe { windows::Win32::Foundation::GetHandleInformation(held.handle, &mut flags) }.unwrap();
    assert_eq!(flags & 1, 0);
    let other = name.clone();
    std::thread::spawn(move || {
        assert_eq!(
            acquire_named(&other, 25).err().unwrap().code,
            "CREDENTIAL_UNAVAILABLE"
        )
    })
    .join()
    .unwrap();
    drop(held);
    std::thread::spawn(move || assert!(acquire_named(&name, 100).is_ok()))
        .join()
        .unwrap();
}

struct OriginalChild {
    child: Child,
    root: PathBuf,
}
impl OriginalChild {
    fn wait(&mut self) {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            if Instant::now() >= end {
                self.child.kill().unwrap();
                let _ = self.child.wait();
                panic!("original mutex child timed out");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for OriginalChild {
    fn drop(&mut self) {
        let _ = std::fs::write(self.root.join("abandon"), b"original release");
        let end = Instant::now() + Duration::from_secs(5);
        while Instant::now() < end {
            if self.child.try_wait().is_ok_and(|status| status.is_some()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[test]
fn abandoned_gate_is_rejected_and_ownership_is_released_on_the_observing_thread() {
    const MODE: &str = "QQ_CLI_TEST_ABANDONED_VAULT_GATE";
    if let Some(root) = std::env::var_os(MODE) {
        let root = PathBuf::from(root);
        let name = std::fs::read_to_string(root.join("name")).unwrap();
        let _held = acquire_named(&name, 500).unwrap();
        std::fs::write(root.join("ready"), b"original held").unwrap();
        let end = Instant::now() + Duration::from_secs(5);
        while !root.join("abandon").exists() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(5));
        }
        // Intentionally skip Rust destructors in this original child to exercise native abandonment.
        std::process::exit(0);
    }
    let temp = tempfile::tempdir().unwrap();
    let name = name();
    let _keeper = open(&name).unwrap();
    std::fs::write(temp.path().join("name"), &name).unwrap();
    let child=Command::new(std::env::current_exe().unwrap()).args(["--exact","native::credential_lock::tests::abandoned_gate_is_rejected_and_ownership_is_released_on_the_observing_thread","--test-threads=1"])
        .env(MODE,temp.path()).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let mut child = OriginalChild {
        child,
        root: temp.path().into(),
    };
    let end = Instant::now() + Duration::from_secs(3);
    while !temp.path().join("ready").exists() && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(temp.path().join("ready").exists());
    assert!(acquire_named(&name, 25).is_err());
    std::fs::write(temp.path().join("abandon"), b"original abandon").unwrap();
    child.wait();
    assert_eq!(
        acquire_named(&name, 100).err().unwrap().code,
        "CREDENTIAL_UNAVAILABLE"
    );
    std::thread::spawn(move || assert!(acquire_named(&name, 100).is_ok()))
        .join()
        .unwrap();
}

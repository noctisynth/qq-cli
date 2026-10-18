#![cfg(windows)]
use qq_cli::storage::credentials::{Credentials, SystemCredentials};

#[test]
fn independent_vault_namespaces_survive_concurrent_delete_and_restore() {
    const MODE: &str = "QQ_CLI_TEST_CREDENTIAL_ABSENCE";
    if let Some(root) = std::env::var_os(MODE) {
        let credentials = SystemCredentials::new(std::path::Path::new(&root)).unwrap();
        match credentials.get("synthetic-restore") {
            Err(error) if error.code == "CREDENTIAL_NOT_FOUND" => (),
            Err(error) => panic!("original absence reader error: {}", error.code),
            Ok(_) => panic!("original absence reader unexpectedly saw a credential"),
        }
        return;
    }
    std::thread::scope(|scope| {
        let workers = (0..4)
            .map(|worker| {
                scope.spawn(move || {
                    let temp = tempfile::tempdir().unwrap();
                    let credentials = SystemCredentials::new(temp.path()).unwrap();
                    struct Cleanup(SystemCredentials);
                    impl Drop for Cleanup {
                        fn drop(&mut self) {
                            let _ = self.0.delete("synthetic-restore");
                        }
                    }
                    let credentials = Cleanup(credentials);
                    for cycle in 0..20 {
                        credentials
                            .0
                            .set("synthetic-restore", "original-synthetic-value")
                            .unwrap();
                        assert_eq!(
                            credentials.0.get("synthetic-restore").unwrap(),
                            "original-synthetic-value"
                        );
                        credentials.0.delete("synthetic-restore").unwrap();
                        assert_eq!(
                            credentials.0.get("synthetic-restore").err().unwrap().code,
                            "CREDENTIAL_NOT_FOUND"
                        );
                        let mut child = std::process::Command::new(
                            std::env::current_exe().unwrap(),
                        )
                        .args([
                            "--exact",
                            "independent_vault_namespaces_survive_concurrent_delete_and_restore",
                            "--test-threads=1",
                        ])
                        .env(MODE, temp.path())
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                        .unwrap();
                        let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
                        loop {
                            if let Some(status) = child.try_wait().unwrap() {
                                let output = child.wait_with_output().unwrap();
                                assert!(
                                    status.success(),
                                    "original absence reader failed: {} {}",
                                    String::from_utf8_lossy(&output.stdout),
                                    String::from_utf8_lossy(&output.stderr)
                                );
                                break;
                            }
                            if std::time::Instant::now() >= end {
                                child.kill().unwrap();
                                let _ = child.wait();
                                panic!("original absence reader timed out");
                            }
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        credentials
                            .0
                            .set("synthetic-restore", "original-synthetic-value")
                            .unwrap();
                        let restored = credentials.0.get("synthetic-restore");
                        assert!(
                            restored
                                .as_ref()
                                .is_ok_and(|value| value == "original-synthetic-value"),
                            "worker {worker}, cycle {cycle}, restored read error: {}",
                            restored
                                .err()
                                .map(|error| error.code)
                                .unwrap_or("VALUE_MISMATCH")
                        );
                    }
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
    });
}

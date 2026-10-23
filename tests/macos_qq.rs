#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "common/asar_fixture.rs"]
mod asar_fixture;
use qq_cli::napcat::qq;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn thin(kind: u32) -> Vec<u8> {
    [0xfeedfacf_u32, 0x0100000c, 0, kind, 0, 0, 0, 0]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect()
}
fn fat(binary: &[u8], wide: bool) -> Vec<u8> {
    let mut bytes = vec![0_u8; 64];
    bytes[..4].copy_from_slice(&(if wide { 0xcafebabf_u32 } else { 0xcafebabe_u32 }).to_be_bytes());
    bytes[4..8].copy_from_slice(&1_u32.to_be_bytes());
    bytes[8..12].copy_from_slice(&0x0100000c_u32.to_be_bytes());
    if wide {
        bytes[16..24].copy_from_slice(&64_u64.to_be_bytes());
        bytes[24..32].copy_from_slice(&(binary.len() as u64).to_be_bytes());
        bytes[32..36].copy_from_slice(&6_u32.to_be_bytes());
    } else {
        bytes[16..20].copy_from_slice(&64_u32.to_be_bytes());
        bytes[20..24].copy_from_slice(&(binary.len() as u32).to_be_bytes());
        bytes[24..28].copy_from_slice(&6_u32.to_be_bytes());
    }
    bytes.extend_from_slice(binary);
    bytes
}
fn fixture(base: &Path) -> PathBuf {
    let contents = base.join("QQ.app/Contents");
    let app = contents.join("Resources/app");
    std::fs::create_dir_all(&app).unwrap();
    std::fs::create_dir(contents.join("MacOS")).unwrap();
    let executable = contents.join("MacOS/QQ");
    std::fs::write(&executable, thin(2)).unwrap();
    std::fs::write(app.join("major.node"), thin(8)).unwrap();
    std::fs::write(app.join("wrapper.node"), thin(6)).unwrap();
    std::fs::write(contents.join("Info.plist"), b"synthetic metadata bytes").unwrap();
    asar_fixture::write(&app, serde_json::json!({"offset":"0","size":4}), b"test");
    std::fs::write(app.join("package.json"),serde_json::to_vec(&serde_json::json!({"name":"qq-chat","platform":"darwin","version":"7.0.2-53644","buildVersion":"53644","main":"./application.asar/app_launcher/index.js","eleArch":"x64"})).unwrap()).unwrap();
    executable
}
fn inspect(path: &Path) -> qq::Application {
    qq::inspect(path, Instant::now() + Duration::from_secs(5)).unwrap()
}

#[test]
fn thin_fat32_and_fat64_native_slices_ignore_misleading_package_architecture() {
    for binary in [thin(2), fat(&thin(2), false), fat(&thin(2), true)] {
        let temp = tempfile::tempdir().unwrap();
        let executable = fixture(&temp.path().canonicalize().unwrap());
        std::fs::write(&executable, binary).unwrap();
        let result = inspect(&executable);
        assert_eq!(result.architecture, "aarch64");
        assert_eq!(result.build, "53644");
        assert_eq!(result.version, "7.0.2-53644");
        assert_eq!(result.fingerprint, inspect(&executable).fingerprint);
    }
}

#[test]
fn rejects_wrong_architecture_types_and_malformed_fat_or_load_commands() {
    let mut cases = Vec::new();
    let mut intel = thin(2);
    intel[4..8].copy_from_slice(&0x01000007_u32.to_le_bytes());
    cases.push(intel);
    cases.push(thin(6));
    cases.push(vec![0; 31]);
    let mut bad = thin(2);
    bad[16..20].copy_from_slice(&1_u32.to_le_bytes());
    cases.push(bad);
    let mut bad = thin(2);
    bad[20..24].copy_from_slice(&8_u32.to_le_bytes());
    cases.push(bad);
    let mut bad = thin(2);
    bad[8..12].copy_from_slice(&2_u32.to_le_bytes());
    cases.push(bad);
    let mut bad = fat(&thin(2), false);
    bad[16..20].copy_from_slice(&16_u32.to_be_bytes());
    cases.push(bad);
    let mut bad = fat(&thin(2), false);
    bad[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    cases.push(bad);
    let mut bad = fat(&thin(2), true);
    bad[16..24].copy_from_slice(&u64::MAX.to_be_bytes());
    cases.push(bad);
    let mut bad = fat(&thin(2), true);
    bad[36] = 1;
    cases.push(bad);
    let mut bad = fat(&thin(2), false);
    bad[12..16].copy_from_slice(&1_u32.to_be_bytes());
    cases.push(bad);
    let mut bad = fat(&thin(2), false);
    bad[4..8].copy_from_slice(&2_u32.to_be_bytes());
    let duplicate = bad[8..28].to_vec();
    bad[28..48].copy_from_slice(&duplicate);
    cases.push(bad);
    for binary in cases {
        let temp = tempfile::tempdir().unwrap();
        let executable = fixture(&temp.path().canonicalize().unwrap());
        std::fs::write(&executable, binary).unwrap();
        assert!(qq::inspect(&executable, Instant::now() + Duration::from_secs(5)).is_err());
    }
}

#[test]
fn all_selected_files_affect_fingerprint_and_paths_are_not_ownership() {
    for leaf in [
        "Resources/app/wrapper.node",
        "Resources/app/major.node",
        "Resources/app/application.asar",
        "Resources/app/package.json",
        "Info.plist",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let executable = fixture(&temp.path().canonicalize().unwrap());
        let original = inspect(&executable);
        let path = executable.parent().unwrap().parent().unwrap().join(leaf);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.push(b' ');
        std::fs::write(&path, bytes).unwrap();
        let changed = inspect(&executable);
        assert_ne!(original.fingerprint, changed.fingerprint);
        assert_eq!(original.id, changed.id);
    }
}

#[test]
fn missing_linked_mismatched_and_modified_entry_metadata_are_rejected() {
    for variant in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let executable = fixture(&base);
        let app = base.join("QQ.app/Contents/Resources/app");
        match variant {
            0 => std::fs::remove_file(app.join("wrapper.node")).unwrap(),
            1 => {
                std::fs::rename(app.join("major.node"), base.join("saved")).unwrap();
                std::os::unix::fs::symlink(base.join("saved"), app.join("major.node")).unwrap();
            }
            2 => std::fs::hard_link(&executable, base.join("linked")).unwrap(),
            v => {
                let path = app.join("package.json");
                let mut data: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                data[if v == 3 { "buildVersion" } else { "main" }] = serde_json::json!("changed");
                std::fs::write(path, serde_json::to_vec(&data).unwrap()).unwrap();
            }
        }
        assert!(qq::inspect(&executable, Instant::now() + Duration::from_secs(5)).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture(&temp.path().canonicalize().unwrap());
    assert!(qq::inspect(&executable, Instant::now()).is_err());
}

#[test]
fn structurally_valid_synthetic_app_is_not_a_signed_original() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture(&temp.path().canonicalize().unwrap());
    inspect(&executable);
    assert!(
        qq_cli::native::macos_signature::verify_original(
            executable
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap(),
            Instant::now() + Duration::from_secs(5)
        )
        .is_err()
    );
}

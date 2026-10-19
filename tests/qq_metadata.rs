use qq_cli::{napcat::qq, native::platform};
use std::time::{Duration, Instant};

#[path = "common/asar_fixture.rs"]
mod asar_fixture;
#[path = "common/qq_fixture.rs"]
mod fixture;
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn asar_entry_is_verified_in_both_layouts_and_entire_archive_changes_invalidate_it() {
    for alternate in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let executable = fixture::fixture(temp.path(), alternate);
        let app = if alternate {
            temp.path()
                .join("resources/app/versions")
                .join(fixture::VERSION)
        } else {
            temp.path()
                .join("versions")
                .join(fixture::VERSION)
                .join("resources/app")
        };
        asar_fixture::write(&app, serde_json::json!({"offset":"0","size":4}), b"test");
        let first = qq::inspect(&executable, deadline()).unwrap();
        asar_fixture::write(&app, serde_json::json!({"offset":"0","size":4}), b"next");
        let next = qq::inspect(&executable, deadline()).unwrap();
        assert_ne!(first.entry_sha256, next.entry_sha256);
        assert_ne!(first.fingerprint, next.fingerprint);
    }
}

#[test]
fn asar_links_unpacked_missing_and_out_of_bounds_entries_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(temp.path(), false);
    let app = temp
        .path()
        .join("versions")
        .join(fixture::VERSION)
        .join("resources/app");
    for leaf in [
        serde_json::json!({"link":"other.js"}),
        serde_json::json!({"unpacked":true,"size":4}),
        serde_json::json!({"offset":"18446744073709551615","size":4}),
        serde_json::json!({"offset":"0","size":5}),
        serde_json::json!({"files":{}}),
    ] {
        asar_fixture::write(&app, leaf, b"test");
        assert_eq!(
            qq::inspect(&executable, deadline()).unwrap_err().code,
            "QQ_METADATA_UNVERIFIED"
        );
    }
    for bytes in [
        vec![],
        vec![0; 16],
        [4_u32, u32::MAX, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect(),
    ] {
        std::fs::write(app.join("application.asar"), bytes).unwrap();
        assert_eq!(
            qq::inspect(&executable, deadline()).unwrap_err().code,
            "QQ_METADATA_UNVERIFIED"
        );
    }
}

#[test]
fn both_recorded_layouts_are_inspected_without_exposing_local_paths_or_unknown_fields() {
    for alternate in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let executable = fixture::fixture(temp.path(), alternate);
        let application = qq::inspect(&executable, deadline()).unwrap();
        assert_eq!(application.version, fixture::VERSION);
        assert_eq!(application.build, "60001");
        assert_eq!(application.architecture, "x86_64");
        let text = serde_json::to_string(&application).unwrap();
        assert!(!text.contains(temp.path().to_str().unwrap()));
        assert!(!text.contains("synthetic omitted field"));
    }
}

#[test]
fn entry_native_payload_and_metadata_changes_invalidate_the_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(temp.path(), false);
    let initial = qq::inspect(&executable, deadline()).unwrap();
    let app = temp
        .path()
        .join("versions")
        .join(fixture::VERSION)
        .join("resources/app");
    std::fs::write(app.join("launcher.js"), b"changed synthetic entry").unwrap();
    let entry = qq::inspect(&executable, deadline()).unwrap();
    assert_ne!(entry.fingerprint, initial.fingerprint);
    let mut payload = fixture::pe(0x8664);
    payload[127] = 1;
    std::fs::write(
        temp.path()
            .join("versions")
            .join(fixture::VERSION)
            .join("major.node"),
        payload,
    )
    .unwrap();
    let native = qq::inspect(&executable, deadline()).unwrap();
    assert_ne!(native.fingerprint, entry.fingerprint);
    std::fs::write(
        app.join("package.json"),
        r#"{"version":"9.9.99.60001","main":"launcher.js"}"#,
    )
    .unwrap();
    let metadata = qq::inspect(&executable, deadline()).unwrap();
    assert_eq!(metadata.build, "60001");
    assert_ne!(metadata.fingerprint, native.fingerprint);
}

#[test]
fn missing_disagreeing_or_traversing_versions_never_receive_a_default_build() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(temp.path(), false);
    let config = temp.path().join("versions/config.json");
    std::fs::write(&config, r#"{"curVersion":"../outside"}"#).unwrap();
    assert_eq!(
        qq::inspect(&executable, deadline()).unwrap_err().code,
        "QQ_METADATA_UNVERIFIED"
    );
    fixture::fixture(temp.path(), false);
    let package = temp
        .path()
        .join("versions")
        .join(fixture::VERSION)
        .join("resources/app/package.json");
    std::fs::write(
        package,
        r#"{"version":"9.9.99","buildVersion":"60002","main":"launcher.js"}"#,
    )
    .unwrap();
    assert_eq!(
        qq::inspect(&executable, deadline()).unwrap_err().code,
        "QQ_VERSION_MISMATCH"
    );
    std::fs::remove_file(config).unwrap();
    assert_eq!(
        qq::inspect(&executable, deadline()).unwrap_err().code,
        "QQ_METADATA_UNVERIFIED"
    );
}

#[test]
fn malformed_pe_architecture_mismatch_and_expired_hash_deadline_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let executable = fixture::fixture(temp.path(), false);
    let major = temp
        .path()
        .join("versions")
        .join(fixture::VERSION)
        .join("major.node");
    std::fs::write(&major, fixture::pe(0xaa64)).unwrap();
    assert_eq!(
        qq::inspect(&executable, deadline()).unwrap_err().code,
        "QQ_ARCHITECTURE_MISMATCH"
    );
    let mut invalid = fixture::pe(0x8664);
    invalid[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&executable, invalid).unwrap();
    assert_eq!(
        qq::pe_architecture(&executable).unwrap_err().code,
        "QQ_METADATA_UNVERIFIED"
    );
    assert_eq!(
        qq::hash_file(&executable, 1024, Instant::now())
            .unwrap_err()
            .code,
        "DEADLINE"
    );
    assert_eq!(
        qq::hash_file(&executable, 10, deadline()).unwrap_err().code,
        "QQ_METADATA_UNVERIFIED"
    );
}

#[test]
fn uninstall_location_parsing_never_executes_arguments_or_accepts_an_unrelated_program() {
    let temp = tempfile::tempdir().unwrap();
    let uninstall = temp.path().join("Uninstall.exe");
    let expected = temp.path().join("QQ.exe");
    for command in [
        format!("\"{}\" --synthetic-unused-argument", uninstall.display()),
        uninstall.to_string_lossy().into_owned(),
    ] {
        assert_eq!(
            platform::executable_from_uninstall(&command).unwrap(),
            expected
        );
    }
    assert!(platform::executable_from_uninstall("relative-uninstall.exe").is_err());
    assert!(
        platform::executable_from_uninstall(&format!(
            "\"{}\"",
            temp.path().join("unrelated.exe").display()
        ))
        .is_err()
    );
}

#[cfg(windows)]
#[test]
fn native_host_probe_reads_version_and_native_architecture_without_application_access() {
    let host = platform::host();
    assert_eq!(host.os, "windows");
    assert!(host.os_build.is_some_and(|build| build > 0));
    assert!(!host.native_arch.is_empty());
    assert_eq!(host.process_arch, std::env::consts::ARCH);
}

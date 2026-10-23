use super::*;
use serde_json::json;

#[test]
fn legacy_windows_identity_preserves_exact_wire_and_rejects_foreign_host() {
    let serialized = r#"{"processId":42,"created":133000000000000001,"image":"C:\\QQ\\QQ.exe"}"#;
    let wire: serde_json::Value = serde_json::from_str(serialized).unwrap();
    let identity: Identity = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&identity).unwrap(), wire);
    assert_eq!(serde_json::to_string(&identity).unwrap(), serialized);
    #[cfg(not(windows))]
    assert_eq!(
        identity.validate_for_host().unwrap_err().code,
        "PROCESS_PLATFORM_MISMATCH"
    );
}
fn macos() -> serde_json::Value {
    json!({"platform":"macos","bootSession":"00000000-0000-0000-0000-000000000001","processId":42,"startSeconds":1700000000,"startMicroseconds":999999,"image":"/synthetic/QQ"})
}
#[test]
fn platform_tags_never_guess_or_mix_timestamp_units() {
    let wire = macos();
    let identity: Identity = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&identity).unwrap(), wire);
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    identity.validate_for_host().unwrap();
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    assert_eq!(
        identity.validate_for_host().unwrap_err().code,
        "PROCESS_PLATFORM_MISMATCH"
    );
    for name in [
        "platform",
        "bootSession",
        "startSeconds",
        "startMicroseconds",
    ] {
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove(name);
        assert!(serde_json::from_value::<Identity>(missing).is_err());
    }
    for (name, value) in [
        ("platform", json!("linux")),
        ("created", json!(1)),
        ("extra", json!(true)),
    ] {
        let mut corrupt = wire.clone();
        corrupt[name] = value;
        assert!(serde_json::from_value::<Identity>(corrupt).is_err());
    }
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn invalid_macos_identity_is_rejected_before_native_observation() {
    for (name, value) in [
        ("startMicroseconds", json!(1000000)),
        ("startSeconds", json!(0)),
        ("processId", json!(0)),
        ("processId", json!(u32::MAX)),
        ("image", json!("relative")),
        ("bootSession", json!("not-a-boot-id")),
    ] {
        let mut corrupt = macos();
        corrupt[name] = value;
        let identity: Identity = serde_json::from_value(corrupt).unwrap();
        assert_eq!(
            crate::native::process::Pinned::reopen(
                &identity,
                std::time::Instant::now() + std::time::Duration::from_secs(2)
            )
            .err()
            .unwrap()
            .code,
            "PROCESS_IDENTITY_CHANGED"
        );
    }
}

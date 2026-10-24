//! Platform artifact scope, independent of online compatibility observations.
use crate::{
    core::error::{AppError, Result},
    napcat::install::catalog::{self, Release},
};

pub const DEFAULT_VERSION: &str = "4.18.41";

pub fn lookup(version: &str) -> Result<Release> {
    match version.strip_prefix('v').unwrap_or(version) {
        "4.18.41" | "4.18.38" => catalog::lookup(version),
        _ => Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "This Framework version is outside the reviewed macOS arm64 artifact catalog.",
            4,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_versions_pin_all_native_arm64_members_without_accepting_other_windows_releases() {
        for version in ["4.18.38", "v4.18.41"] {
            let release = lookup(version).unwrap();
            for name in [
                "package.json",
                "native/napi2native/napi2native.darwin.arm64.node",
                "native/packet/MoeHoo.darwin.arm64.node",
                "native/ffmpeg/ffmpegAddon.darwin.arm64.node",
            ] {
                assert!(release.archive.required.iter().any(|entry| entry.0 == name));
            }
        }
        assert_eq!(lookup("4.18.37").err().unwrap().code, "VERSION_UNSUPPORTED");
    }
}

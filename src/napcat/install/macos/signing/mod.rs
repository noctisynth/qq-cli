//! Fixed entitlement templates and read-only post-sign verification.
//! These observations grant neither copy ownership nor permission to launch.
use crate::core::error::Result;
use crate::native::macos_signing_info::{invalid, verify};
use crate::storage::state::reject_links;
use core_foundation::url::CFURL;
use security_framework::os::macos::code_signing::{Flags, SecRequirement, SecStaticCode};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Instant;

mod worker;
pub(super) use worker::worker_entry;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Only the main QQ process loading the reviewed native hook.
    Main,
    /// Explicitly reviewed Electron hosts/helpers, not every nested executable.
    Electron,
    /// Explicitly reviewed native plugin host; not inferred from source entitlements.
    PluginHost,
    /// Native code without dynamic-code/native-hook exceptions.
    Native,
}

impl Policy {
    fn keys(self) -> &'static [&'static str] {
        match self {
            Self::Main => &[
                "com.apple.security.cs.allow-jit",
                "com.apple.security.cs.disable-library-validation",
                "com.apple.security.cs.allow-unsigned-executable-memory",
            ],
            Self::Electron | Self::PluginHost => &[
                "com.apple.security.cs.allow-jit",
                "com.apple.security.cs.disable-library-validation",
            ],
            Self::Native => &[],
        }
    }

    /// Fresh complete policy, never a merge with source App Sandbox/shared
    /// groups/get-task-allow or arbitrary caller-provided entitlement keys.
    pub fn entitlements(self) -> String {
        let mut xml = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict>",
        );
        for key in self.keys() {
            xml.push_str(&format!("<key>{key}</key><true/>"));
        }
        xml.push_str("</dict></plist>\n");
        xml
    }

    /// Synchronous OS verification, with cooperative deadline checks. A future
    /// signing worker must isolate this call and bind it to the full inventory.
    /// Exact entitlements apply to native arm64; this is not a fat-slice audit.
    pub fn verify(self, path: &Path, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        reject_links(path)?;
        if !path.is_absolute() || path.canonicalize().map_err(|_| invalid())? != path {
            return Err(invalid());
        }
        let before = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
        if !before.is_dir() && !before.is_file() {
            return Err(invalid());
        }
        let url = CFURL::from_path(path, before.is_dir()).ok_or_else(invalid)?;
        let code = SecStaticCode::from_path(&url, Flags::NONE).map_err(|_| invalid())?;
        let requirement: SecRequirement = "true".parse().map_err(|_| invalid())?;
        code.check_validity(
            Flags::STRICT_VALIDATE
                | Flags::CHECK_NESTED_CODE
                | Flags::CHECK_ALL_ARCHITECTURES
                | Flags::NO_NETWORK_ACCESS,
            &requirement,
        )
        .map_err(|_| invalid())?;
        verify(&code, self.keys())?;
        crate::core::deadline::check(deadline)?;
        reject_links(path)?;
        let after = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
        let stamp = |m: &std::fs::Metadata| {
            (
                m.dev(),
                m.ino(),
                m.mode(),
                m.uid(),
                m.gid(),
                m.len(),
                m.mtime(),
                m.mtime_nsec(),
                m.ctime(),
                m.ctime_nsec(),
                m.nlink(),
            )
        };
        if stamp(&before) != stamp(&after) {
            return Err(invalid());
        }
        Ok(())
    }
}

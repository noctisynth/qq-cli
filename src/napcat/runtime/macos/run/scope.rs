//! Declaration emitted only by the fixed sandbox launcher; not PID ownership.
use super::conflict;
use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scope {
    schema_version: u8,
    app: PathBuf,
    profile_sha256: String,
}
impl Scope {
    pub(in crate::napcat::runtime::macos) fn app(&self) -> &Path {
        &self.app
    }
    pub(in crate::napcat::runtime::macos) fn declare(app: PathBuf, profile: &str) -> Self {
        Self {
            schema_version: 1,
            app,
            profile_sha256: format!("{:x}", Sha256::digest(profile.as_bytes())),
        }
    }
    pub(in crate::napcat::runtime::macos) fn validate(&self, app: &Path) -> Result<()> {
        if self.schema_version != 1
            || self.app != app
            || !self.app.is_absolute()
            || self.profile_sha256.len() != 64
            || !self
                .profile_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(conflict());
        }
        Ok(())
    }
}

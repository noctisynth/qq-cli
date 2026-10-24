//! Private, platform-specific observations. Legacy Windows bytes remain unchanged.
use crate::core::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Identity(Wire);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Wire {
    Windows(Windows),
    Macos(Macos),
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Windows {
    process_id: u32,
    created: u64,
    image: PathBuf,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Macos {
    platform: MacosTag,
    boot_session: String,
    process_id: u32,
    start_seconds: u64,
    start_microseconds: u64,
    image: PathBuf,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
enum MacosTag {
    #[serde(rename = "macos")]
    Macos,
}
impl Identity {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) fn macos_generation(&self) -> Result<(u64, u64, &str)> {
        self.validate_for_host()?;
        match &self.0 {
            Wire::Macos(v) => Ok((v.start_seconds, v.start_microseconds, &v.boot_session)),
            _ => Err(platform()),
        }
    }
    #[cfg(windows)]
    pub(crate) fn windows(process_id: u32, created: u64, image: PathBuf) -> Self {
        Self(Wire::Windows(Windows {
            process_id,
            created,
            image,
        }))
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub(crate) fn macos(
        process_id: u32,
        seconds: u64,
        microseconds: u64,
        image: PathBuf,
        boot_session: String,
    ) -> Self {
        Self(Wire::Macos(Macos {
            platform: MacosTag::Macos,
            boot_session,
            process_id,
            start_seconds: seconds,
            start_microseconds: microseconds,
            image,
        }))
    }
    pub fn process_id(&self) -> u32 {
        match &self.0 {
            Wire::Windows(v) => v.process_id,
            Wire::Macos(v) => v.process_id,
        }
    }
    pub fn image(&self) -> &Path {
        match &self.0 {
            Wire::Windows(v) => &v.image,
            Wire::Macos(v) => &v.image,
        }
    }
    /// Legacy Windows FILETIME, available only to Windows callers.
    #[cfg(windows)]
    pub fn created(&self) -> u64 {
        match &self.0 {
            Wire::Windows(v) => v.created,
            Wire::Macos(_) => 0,
        }
    }
    pub(crate) fn validate_for_host(&self) -> Result<()> {
        let valid = match &self.0 {
            Wire::Windows(v) => {
                if !cfg!(windows) {
                    return Err(platform());
                }
                v.created != 0
            }
            Wire::Macos(v) => {
                if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
                    return Err(platform());
                }
                v.start_seconds != 0
                    && v.start_microseconds < 1_000_000
                    && valid_boot_session(&v.boot_session)
            }
        };
        if !valid
            || self.process_id() == 0
            || self.process_id() > i32::MAX as u32 && matches!(self.0, Wire::Macos(_))
            || !self.image().is_absolute()
            || self
                .image()
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control))
        {
            return Err(super::changed());
        }
        Ok(())
    }
}
fn valid_boot_session(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn platform() -> AppError {
    AppError::new(
        "PROCESS_PLATFORM_MISMATCH",
        "Recorded process belongs to another platform; no migration or process probe was attempted.",
        4,
    )
}

#[cfg(test)]
mod tests;

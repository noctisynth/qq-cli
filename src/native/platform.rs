//! Read-only host and installation-location discovery. Never executes an uninstall command.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) mod macos;
use std::path::{Path, PathBuf};

use crate::core::error::{AppError, Result};

pub use crate::core::contracts::diagnostic::Host;

pub fn host() -> Host {
    #[cfg(windows)]
    {
        let architecture = winsafe::GetNativeSystemInfo().wProcessorArchitecture;
        let native = match architecture {
            winsafe::co::PROCESSOR_ARCHITECTURE::AMD64 => "x86_64",
            winsafe::co::PROCESSOR_ARCHITECTURE::ARM64 => "aarch64",
            winsafe::co::PROCESSOR_ARCHITECTURE::INTEL => "x86",
            _ => "unknown",
        };
        Host {
            os: "windows".into(),
            os_build: Some(windows_version::OsVersion::current().build),
            native_arch: native.into(),
            process_arch: std::env::consts::ARCH.into(),
        }
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        Host {
            os: "macos".into(),
            os_build: None,
            // A native arm64 executable cannot run on an Intel Mac or under
            // Rosetta. Do not infer a Darwin/product build number from this fact.
            native_arch: "aarch64".into(),
            process_arch: "aarch64".into(),
        }
    }
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    Host {
        os: std::env::consts::OS.into(),
        os_build: None,
        native_arch: "unknown".into(),
        process_arch: std::env::consts::ARCH.into(),
    }
}

pub fn qq_executables() -> Result<Vec<PathBuf>> {
    #[cfg(windows)]
    {
        let mut candidates = Vec::new();
        for root in [
            windows_registry::CURRENT_USER,
            windows_registry::LOCAL_MACHINE,
        ] {
            for name in [
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\QQ",
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\QQ",
            ] {
                let key = match root.open(name) {
                    Ok(key) => key,
                    Err(error) if matches!(error.code().0 as u32, 0x80070002 | 0x80070003) => {
                        continue;
                    }
                    Err(_) => {
                        return Err(AppError::new(
                            "QQ_DISCOVERY_UNAVAILABLE",
                            "QQ installation registry metadata could not be read.",
                            3,
                        ));
                    }
                };
                let before = candidates.len();
                if let Ok(location) = key.get_string("InstallLocation") {
                    let path = PathBuf::from(location.trim_matches('"')).join("QQ.exe");
                    if path.is_absolute() {
                        candidates.push(path);
                    }
                }
                if let Ok(command) = key.get_string("UninstallString") {
                    candidates.push(executable_from_uninstall(&command)?);
                }
                if candidates.len() == before {
                    return Err(discovery_error());
                }
            }
        }
        Ok(candidates)
    }
    #[cfg(not(windows))]
    Ok(Vec::new())
}

pub fn executable_from_uninstall(command: &str) -> Result<PathBuf> {
    if command.len() > 32768 || command.chars().any(char::is_control) {
        return Err(discovery_error());
    }
    let command = command.trim();
    let executable = if let Some(quoted) = command.strip_prefix('"') {
        let end = quoted.find('"').ok_or_else(discovery_error)?;
        &quoted[..end]
    } else {
        let end = command
            .to_ascii_lowercase()
            .find(".exe")
            .ok_or_else(discovery_error)?
            + 4;
        &command[..end]
    };
    let path = Path::new(executable);
    if !path.is_absolute()
        || !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("uninstall.exe"))
    {
        return Err(discovery_error());
    }
    Ok(path.parent().ok_or_else(discovery_error)?.join("QQ.exe"))
}

fn discovery_error() -> AppError {
    AppError::new(
        "QQ_DISCOVERY_UNRECOGNIZED",
        "QQ registry entry does not identify a supported absolute uninstall executable path.",
        3,
    )
}

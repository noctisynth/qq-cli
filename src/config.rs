use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    pub schema_version: u8,
    pub state_dir: Option<PathBuf>,
    pub request_timeout_seconds: u16,
    pub response_bytes: usize,
    pub scan_limit: usize,
    pub page_limit: usize,
    pub permissions: Permissions,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Permissions {
    pub read: bool,
    pub send: bool,
    pub media_download: bool,
    pub events: bool,
    pub targets: Vec<String>,
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            read: true,
            send: true,
            media_download: true,
            events: true,
            targets: Vec::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            state_dir: None,
            request_timeout_seconds: 10,
            response_bytes: 4 * 1024 * 1024,
            scan_limit: 1000,
            page_limit: 100,
            permissions: Permissions::default(),
        }
    }
}

pub struct Settings {
    pub config: Config,
    pub state_dir: PathBuf,
}

impl Settings {
    /// Merely resolving settings never creates files or enumerates QQ data.
    pub fn load(explicit: Option<&Path>) -> Result<Self> {
        let override_path = explicit
            .map(Path::to_owned)
            .or_else(|| env::var_os("QQ_CLI_CONFIG").map(PathBuf::from));
        let default_root = if override_path.is_none() {
            default_state_root()?
        } else {
            PathBuf::new()
        };
        let path = override_path
            .clone()
            .unwrap_or_else(|| default_root.join("config.json"));
        let config: Config = match read_json(&path, 1024 * 1024) {
            Ok(config) => config,
            Err(error) if error.code == "FILE_NOT_FOUND" && override_path.is_none() => {
                Config::default()
            }
            Err(_) => {
                return Err(AppError::new(
                    "CONFIG_INVALID",
                    "Configuration is missing, unreadable or invalid.",
                    2,
                ));
            }
        };
        if config.schema_version != 1
            || !(1..=60).contains(&config.request_timeout_seconds)
            || !(1024..=4 * 1024 * 1024).contains(&config.response_bytes)
            || !(1..=100_000).contains(&config.scan_limit)
            || !(1..=1000).contains(&config.page_limit)
        {
            return Err(AppError::new(
                "CONFIG_INVALID",
                "Configuration version or resource limits are invalid.",
                2,
            ));
        }
        let state_dir = match &config.state_dir {
            Some(root) if root.is_absolute() => root.clone(),
            Some(root) => path.parent().unwrap_or(Path::new(".")).join(root),
            None if override_path.is_some() => {
                path.parent().unwrap_or(Path::new(".")).join("state")
            }
            None => default_root,
        };
        Ok(Self { config, state_dir })
    }
}

fn default_state_root() -> Result<PathBuf> {
    #[cfg(windows)]
    let base = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.map(|base| base.join("qq-cli")).ok_or_else(|| {
        AppError::new(
            "CONFIG_REQUIRED",
            "A user data location or explicit configuration is required.",
            2,
        )
    })
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path, max_bytes: usize) -> Result<T> {
    let file = File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::new(
                "FILE_NOT_FOUND",
                "The requested local file does not exist.",
                2,
            )
        } else {
            AppError::storage()
        }
    })?;
    let mut data = Vec::new();
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| AppError::storage())?;
    if data.len() > max_bytes {
        return Err(AppError::new(
            "FILE_LIMIT",
            "Local JSON exceeds its size limit.",
            2,
        ));
    }
    serde_json::from_slice(&data)
        .map_err(|_| AppError::new("STATE_INVALID", "Local JSON is invalid or incompatible.", 6))
}

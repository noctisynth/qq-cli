//! Read-only preparation. No publication, signing, process creation or ownership grant.
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;

const HOME: &str = "profile-home/Library/Containers/com.tencent.qq/Data";
pub(crate) fn layout_directories() -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();
    for path in [
        "Desktop",
        "Documents",
        "Downloads",
        "Music",
        "Pictures",
        "Movies",
        "Library/Application Support/QQ",
    ]
    .map(|leaf| Path::new(HOME).join(leaf))
    .into_iter()
    .chain(
        [
            "temp",
            "logs",
            "config",
            "cache",
            "napcat-data/config",
            "napcat-data/logs",
            "napcat-data/cache",
            "napcat-data/plugins",
        ]
        .map(PathBuf::from),
    ) {
        for parent in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
            paths.insert(parent.to_owned());
        }
    }
    paths.into_iter().collect()
}

/// Inputs must already be authenticated under the lifecycle lock. Filesystem
/// observations here do not close replacement races or establish provenance.
pub struct Prepared {
    root: PathBuf,
    framework: PathBuf,
    home: PathBuf,
    user_data: PathBuf,
}

impl Prepared {
    pub fn inspect(instance: &Path, framework: &Path) -> Result<Self> {
        let root = directory(instance)?;
        let framework = directory(framework)?;
        if root.parent().is_none()
            || framework.parent().is_none()
            || root.starts_with(&framework)
            || framework.starts_with(&root)
        {
            return Err(invalid());
        }
        let home = root.join(HOME);
        let user_data = home.join("Library/Application Support/QQ");
        for relative in layout_directories() {
            directory(&root.join(relative))?;
        }
        Ok(Self {
            root,
            framework,
            home,
            user_data,
        })
    }

    /// A fresh map for Command::env_clear + envs. Never merge inherited values.
    /// GUI/session compatibility and sandbox enforcement need launch validation.
    pub fn environment(&self) -> BTreeMap<OsString, OsString> {
        let mut values = BTreeMap::new();
        for (key, value) in [
            ("HOME", self.root.join("profile-home")),
            ("CFFIXED_USER_HOME", self.root.join("profile-home")),
            ("TMPDIR", self.root.join("temp")),
            ("XDG_CONFIG_HOME", self.root.join("config")),
            ("XDG_CACHE_HOME", self.root.join("cache")),
            ("NAPCAT_WORKDIR", self.root.join("napcat-data")),
        ] {
            values.insert(key.into(), value.into_os_string());
        }
        values.insert("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into());
        values.insert("LANG".into(), "en_US.UTF-8".into());
        values
    }

    pub fn executable(&self) -> PathBuf {
        self.root.join("QQ.app/Contents/MacOS/QQ")
    }

    pub fn current_directory(&self) -> &Path {
        &self.root
    }

    pub fn electron_home(&self) -> &Path {
        &self.home
    }

    /// One literal argument; no shell or inherited Electron switches.
    pub fn arguments(&self) -> [OsString; 1] {
        let mut value = OsString::from("--user-data-dir=");
        value.push(&self.user_data);
        [value]
    }

    /// Publish beside the exact embedded entry.cjs and bridge.cjs, only after
    /// authenticating the copy, fixed versions and all paths under the lock.
    pub fn main_source(&self) -> Result<String> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Config<'a> {
            instance_root: &'a Path,
            framework_root: &'a Path,
        }
        let config = serde_json::to_string(&Config {
            instance_root: &self.root,
            framework_root: &self.framework,
        })
        .map_err(|_| invalid())?;
        // JSON serialization is essential: path text must never become JS code.
        Ok(format!(
            r#""use strict";
const config = {config};
const fail = () => {{
  process.exitCode = 1;
  process.stderr.write("QQ_CLI_ADAPTER_START_FAILED\n");
}};
try {{
  require('./entry.cjs').start(config, {{
    fs: require('node:fs'), app: require('electron').app,
    process, load: require, onFailure: fail,
  }});
}} catch {{ fail(); }}
"#
        ))
    }
}

fn directory(path: &Path) -> Result<PathBuf> {
    let text = path.to_str().ok_or_else(invalid)?;
    if !path.is_absolute()
        || text.chars().any(char::is_control)
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(invalid());
    }
    reject_links(path).map_err(|_| invalid())?;
    let canonical = path.canonicalize().map_err(|_| invalid())?;
    if canonical.as_os_str() != path.as_os_str() || !canonical.is_dir() {
        return Err(invalid());
    }
    Ok(canonical)
}

fn invalid() -> AppError {
    AppError::new(
        "LAUNCH_ENV_INVALID",
        "Prepared macOS runtime paths are invalid; no process was started.",
        4,
    )
}

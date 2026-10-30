//! Deterministic child environment construction; this module never starts a process.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;

pub struct Bootstrap {
    pub program: PathBuf,
    pub arguments: [OsString; 3],
    pub current_directory: PathBuf,
}

/// Preserve the fixed upstream native-loader positional ABI, without shell quoting.
/// Path shape is not trust: the caller must verify ownership, hashes, compatibility
/// and user authorization under the lifecycle lock immediately before any spawn.
pub fn bootstrap(
    qq_executable: &Path,
    installation: &Path,
    work_directory: &Path,
) -> Result<Bootstrap> {
    let installation = directory(installation)?;
    let qq = regular(qq_executable, "QQ.exe")?;
    let work = directory(work_directory)?;
    let qq_directory = qq.parent().ok_or_else(invalid)?;
    if work.starts_with(&installation)
        || installation.starts_with(&work)
        || work.starts_with(qq_directory)
        || qq_directory.starts_with(&work)
    {
        return Err(invalid());
    }
    let program = regular(&installation.join("napimain.exe"), "napimain.exe")?;
    let dll = regular(&installation.join("napiloader.dll"), "napiloader.dll")?;
    let entry = regular(&installation.join("nativeLoader.cjs"), "nativeLoader.cjs")?;
    let entry = entry.to_str().ok_or_else(invalid)?;
    // Original launcher script uses forward slashes for this argument. Strip the
    // canonical Win32 local-drive prefix before reproducing that representation.
    let entry = entry
        .strip_prefix("\\\\?\\")
        .unwrap_or(entry)
        .replace('\\', "/");
    Ok(Bootstrap {
        program,
        arguments: [ordinary_local(&qq)?, ordinary_local(&dll)?, entry.into()],
        current_directory: work,
    })
}

/// Select the exact original adapter only when authenticated template 2 material
/// is present. Legacy runs keep their original entry and input hash.
pub fn friend_requests_entry(bootstrap: &mut Bootstrap, work: &Path) -> Result<()> {
    let entry = regular(&work.join("config/friend_loader.cjs"), "friend_loader.cjs")?;
    let entry = entry.to_str().ok_or_else(invalid)?;
    bootstrap.arguments[2] = entry
        .strip_prefix("\\\\?\\")
        .unwrap_or(entry)
        .replace('\\', "/")
        .into();
    Ok(())
}

// Match upstream BAT's ordinary-drive positional ABI after verifying the
// canonical files. Extended Win32 prefixes are not passed into QQ's bootstrap.
fn ordinary_local(path: &Path) -> Result<OsString> {
    let value = path.to_str().ok_or_else(invalid)?;
    Ok(value.strip_prefix("\\\\?\\").unwrap_or(value).into())
}

fn regular(path: &Path, name: &str) -> Result<PathBuf> {
    if !path
        .file_name()
        .and_then(|leaf| leaf.to_str())
        .is_some_and(|leaf| leaf.eq_ignore_ascii_case(name))
    {
        return Err(invalid());
    }
    let parent = directory(path.parent().ok_or_else(invalid)?)?;
    let path = parent.join(path.file_name().ok_or_else(invalid)?);
    reject_links(&path).map_err(|_| invalid())?;
    if !path.is_file() {
        return Err(invalid());
    }
    Ok(path)
}

/// Inputs must be bound to verified runtime ownership and QQ metadata before spawning.
/// This function checks directory shape, not ownership or compatibility.
pub fn environment(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    qq_directory: &Path,
    work_directory: &Path,
) -> Result<BTreeMap<OsString, OsString>> {
    let qq = directory(qq_directory)?;
    let work = directory(work_directory)?;
    if qq.starts_with(&work) || work.starts_with(&qq) {
        return Err(invalid());
    }
    let temporary = directory(&work.join("tmp"))?;
    if !temporary.starts_with(&work) {
        return Err(invalid());
    }
    let mut output = BTreeMap::<OsString, OsString>::new();
    for (key, value) in inherited {
        let Some(key) = key.to_str() else { continue };
        let key = key.to_ascii_uppercase();
        // Everything else is intentionally absent, including NODE_OPTIONS, Electron
        // overrides, proxy variables, inherited PATH and all NAPCAT_* overrides.
        if !matches!(
            key.as_str(),
            "SYSTEMROOT"
                | "WINDIR"
                | "USERPROFILE"
                | "APPDATA"
                | "LOCALAPPDATA"
                | "HOMEDRIVE"
                | "HOMEPATH"
                | "USERNAME"
                | "USERDOMAIN"
                | "PROGRAMDATA"
                | "PROGRAMFILES"
                | "PROGRAMFILES(X86)"
                | "COMMONPROGRAMFILES"
                | "COMMONPROGRAMFILES(X86)"
                | "PROCESSOR_ARCHITECTURE"
                | "NUMBER_OF_PROCESSORS"
        ) {
            continue;
        }
        let text = value
            .to_str()
            .filter(|text| {
                text.len() <= 32 * 1024
                    && !text.chars().any(|c| c == '\0' || c == '\r' || c == '\n')
            })
            .ok_or_else(invalid)?;
        if output
            .get(std::ffi::OsStr::new(&key))
            .is_some_and(|existing| existing != text)
        {
            return Err(invalid());
        }
        output.insert(key.into(), value);
    }
    let system = output
        .get(&OsString::from("SYSTEMROOT"))
        .ok_or_else(invalid)?;
    let system = directory(Path::new(system))?;
    let system32 = directory(&system.join("System32"))?;
    if !system32.starts_with(&system) {
        return Err(invalid());
    }
    if let Some(windir) = output.get(&OsString::from("WINDIR"))
        && directory(Path::new(windir))? != system
    {
        return Err(invalid());
    }
    let ordinary_qq = PathBuf::from(ordinary_local(&qq)?);
    let ordinary_system32 = PathBuf::from(ordinary_local(&system32)?);
    let ordinary_system = PathBuf::from(ordinary_local(&system)?);
    let path = std::env::join_paths([ordinary_qq, ordinary_system32, ordinary_system])
        .map_err(|_| invalid())?;
    output.insert("PATH".into(), path);
    output.insert("SYSTEMROOT".into(), ordinary_local(&system)?);
    output.insert("WINDIR".into(), ordinary_local(&system)?);
    output.insert("TEMP".into(), ordinary_local(&temporary)?);
    output.insert("TMP".into(), ordinary_local(&temporary)?);
    output.insert("NAPCAT_WORKDIR".into(), ordinary_local(&work)?);
    Ok(output)
}

fn directory(path: &Path) -> Result<PathBuf> {
    let text = path.to_string_lossy();
    let local_extended = text.strip_prefix("\\\\?\\").is_some_and(|rest| {
        let bytes = rest.as_bytes();
        bytes.first().is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(1) == Some(&b':')
            && bytes.get(2) == Some(&b'\\')
    });
    if !path.is_absolute()
        || (text.starts_with("\\\\") && !local_extended)
        || text.starts_with("//")
    {
        return Err(invalid());
    }
    reject_links(path).map_err(|_| invalid())?;
    let path = path.canonicalize().map_err(|_| invalid())?;
    if !path.is_dir() {
        return Err(invalid());
    }
    Ok(path)
}
fn invalid() -> AppError {
    AppError::new(
        "LAUNCH_ENV_INVALID",
        "Child environment or isolated runtime directories are invalid; no process was started.",
        4,
    )
}

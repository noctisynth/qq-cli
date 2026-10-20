//! Fingerprint application metadata and selected payload files, never QQ account storage.
mod entry;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::core::domain::identifier;
use crate::core::error::{AppError, Result};
use crate::storage::config::read_json;
use crate::storage::state::reject_links;

pub use crate::core::contracts::diagnostic::Application;

pub fn inspect(executable: &Path, deadline: Instant) -> Result<Application> {
    reject_links(executable)?;
    let executable = executable.canonicalize().map_err(|_| metadata_error())?;
    local_path(&executable)?;
    if !executable
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("QQ.exe"))
    {
        return Err(metadata_error());
    }
    let root = executable.parent().ok_or_else(metadata_error)?;
    let config_path = first_existing(&[
        root.join("versions/config.json"),
        root.join("resources/app/versions/config.json"),
    ])?;
    let config = config_path
        .as_ref()
        .map(|path| software_json(path))
        .transpose()?;
    let current = config
        .as_ref()
        .map(|value| {
            value
                .get("curVersion")
                .and_then(Value::as_str)
                .ok_or_else(metadata_error)
        })
        .transpose()?;
    if let Some(current) = current {
        validate_version(current)?;
    }
    let package_path = if let Some(current) = current {
        first_existing(&[
            root.join("versions")
                .join(current)
                .join("resources/app/package.json"),
            root.join("resources/app/versions")
                .join(current)
                .join("package.json"),
        ])?
    } else {
        first_existing(&[root.join("resources/app/package.json")])?
    }
    .ok_or_else(metadata_error)?;
    let package = software_json(&package_path)?;
    let package_version = package
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(metadata_error)?;
    validate_version(package_version)?;
    let version = current.unwrap_or(package_version);
    let (version_base, embedded_build) = split_version(version)?;
    let (package_base, package_embedded) = split_version(package_version)?;
    let package_build = package
        .get("buildVersion")
        .map(|value| identifier(value).ok_or_else(metadata_error))
        .transpose()?;
    let build = embedded_build
        .map(str::to_owned)
        .or_else(|| package_build.clone())
        .or_else(|| package_embedded.map(str::to_owned))
        .ok_or_else(metadata_error)?;
    if version_base != package_base
        || package_build.as_ref().is_some_and(|value| value != &build)
        || package_embedded.is_some_and(|value| value != build)
    {
        return Err(AppError::new(
            "QQ_VERSION_MISMATCH",
            "Selected QQ version metadata disagrees; compatibility cannot be inferred.",
            3,
        ));
    }
    if build.len() > 10 || build.parse::<u32>().ok().is_none_or(|number| number == 0) {
        return Err(metadata_error());
    }
    let majors = if let Some(current) = current {
        vec![
            root.join("versions").join(current).join("major.node"),
            root.join("versions")
                .join(current)
                .join("resources/app/major.node"),
            root.join("resources/app/versions")
                .join(current)
                .join("major.node"),
        ]
    } else {
        vec![root.join("resources/app/major.node")]
    };
    let major = first_existing(&majors)?.ok_or_else(metadata_error)?;
    let main = package
        .get("main")
        .and_then(Value::as_str)
        .ok_or_else(metadata_error)?
        .replace('\\', "/");
    let entry = entry::resolve(package_path.parent().ok_or_else(metadata_error)?, &main)?;
    reject_links(&entry)?;
    local_path(&entry)?;
    let total = [&executable, &major, &entry]
        .iter()
        .try_fold(0_u64, |sum, path| {
            sum.checked_add(path.metadata().map_err(|_| metadata_error())?.len())
                .ok_or_else(metadata_error)
        })?;
    if total > 1024 * 1024 * 1024 {
        return Err(metadata_error());
    }
    let architecture = pe_architecture(&executable)?;
    if pe_architecture(&major)? != architecture {
        return Err(AppError::new(
            "QQ_ARCHITECTURE_MISMATCH",
            "QQ entry and selected native payload have different architectures.",
            3,
        ));
    }
    let executable_sha256 = hash_file(&executable, 512 * 1024 * 1024, deadline)?;
    let major_sha256 = hash_file(&major, 512 * 1024 * 1024, deadline)?;
    let entry_sha256 = hash_file(&entry, 512 * 1024 * 1024, deadline)?;
    let mut metadata = Sha256::new();
    metadata.update(hash_file(&package_path, 1024 * 1024, deadline)?);
    if let Some(path) = &config_path {
        metadata.update(hash_file(path, 1024 * 1024, deadline)?);
    }
    let metadata_sha256 = format!("{:x}", metadata.finalize());
    let mut fingerprint = Sha256::new();
    for field in [
        version,
        &build,
        &architecture,
        &executable_sha256,
        &major_sha256,
        &entry_sha256,
        &metadata_sha256,
    ] {
        fingerprint.update((field.len() as u64).to_le_bytes());
        fingerprint.update(field.as_bytes());
    }
    if software_json(&package_path)? != package
        || config_path
            .as_ref()
            .map(|path| software_json(path))
            .transpose()?
            != config
    {
        return Err(AppError::new(
            "QQ_INSTALLATION_CHANGED",
            "QQ metadata changed during inspection; no compatibility result was accepted.",
            3,
        ));
    }
    let root_identity = root.to_str().ok_or_else(metadata_error)?.to_lowercase();
    Ok(Application {
        id: format!(
            "qq-{}",
            &format!("{:x}", Sha256::digest(root_identity.as_bytes()))[..16]
        ),
        version: version.into(),
        build,
        architecture,
        fingerprint: format!("{:x}", fingerprint.finalize()),
        executable_sha256,
        major_sha256,
        entry_sha256,
        metadata_sha256,
        executable,
    })
}

pub fn pe_architecture(path: &Path) -> Result<String> {
    reject_links(path)?;
    local_path(path)?;
    let mut file = File::open(path).map_err(|_| metadata_error())?;
    if !file.metadata().map_err(|_| metadata_error())?.is_file() {
        return Err(metadata_error());
    }
    let mut dos = [0_u8; 64];
    file.read_exact(&mut dos).map_err(|_| metadata_error())?;
    if &dos[..2] != b"MZ" {
        return Err(metadata_error());
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().map_err(|_| metadata_error())?) as u64;
    if !(64..=1024 * 1024).contains(&offset) {
        return Err(metadata_error());
    }
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| metadata_error())?;
    let mut header = [0_u8; 6];
    file.read_exact(&mut header).map_err(|_| metadata_error())?;
    if &header[..4] != b"PE\0\0" {
        return Err(metadata_error());
    }
    match u16::from_le_bytes([header[4], header[5]]) {
        0x8664 => Ok("x86_64".into()),
        0xaa64 => Ok("aarch64".into()),
        0x14c => Ok("x86".into()),
        _ => Err(AppError::new(
            "QQ_ARCHITECTURE_UNKNOWN",
            "PE machine type is not recognized.",
            3,
        )),
    }
}

pub fn hash_file(path: &Path, limit: u64, deadline: Instant) -> Result<String> {
    reject_links(path)?;
    local_path(path)?;
    let mut file = File::open(path).map_err(|_| metadata_error())?;
    let metadata = file.metadata().map_err(|_| metadata_error())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(metadata_error());
    }
    let mut size = 0_u64;
    let mut buffer = [0_u8; 65536];
    let mut hash = Sha256::new();
    loop {
        if Instant::now() >= deadline {
            return Err(AppError::new(
                "DEADLINE",
                "Application metadata inspection exceeded its deadline.",
                3,
            ));
        }
        let count = file.read(&mut buffer).map_err(|_| metadata_error())?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > limit {
            return Err(metadata_error());
        }
        hash.update(&buffer[..count]);
    }
    if size != metadata.len() {
        return Err(metadata_error());
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn first_existing(paths: &[PathBuf]) -> Result<Option<PathBuf>> {
    for path in paths {
        reject_links(path)?;
        local_path(path)?;
        if path.try_exists().map_err(|_| metadata_error())? {
            return Ok(Some(path.clone()));
        }
    }
    Ok(None)
}
fn software_json(path: &Path) -> Result<Value> {
    reject_links(path)?;
    local_path(path)?;
    read_json(path, 1024 * 1024).map_err(|_| metadata_error())
}
fn validate_version(value: &str) -> Result<()> {
    split_version(value).map(|_| ())
}
fn split_version(value: &str) -> Result<(&str, Option<&str>)> {
    if value.len() > 48 {
        return Err(metadata_error());
    }
    let (base, build) = value
        .split_once('-')
        .map_or((value, None), |(base, build)| (base, Some(build)));
    let parts = base.split('.').collect::<Vec<_>>();
    if !(3..=4).contains(&parts.len())
        || parts.iter().any(|part| {
            part.is_empty() || part.len() > 10 || !part.bytes().all(|b| b.is_ascii_digit())
        })
        || build.is_some_and(|build| {
            build.is_empty() || build.len() > 10 || !build.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return Err(metadata_error());
    }
    if parts.len() == 4 {
        if build.is_some() {
            return Err(metadata_error());
        }
        let position = base.rfind('.').ok_or_else(metadata_error)?;
        return Ok((&base[..position], Some(&base[position + 1..])));
    }
    Ok((base, build))
}
fn local_path(path: &Path) -> Result<()> {
    #[cfg(windows)]
    if path.components().any(|part|matches!(part,std::path::Component::Prefix(prefix) if matches!(prefix.kind(),std::path::Prefix::UNC(..)|std::path::Prefix::VerbatimUNC(..)))) {return Err(metadata_error());}
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}
fn metadata_error() -> AppError {
    AppError::new(
        "QQ_METADATA_UNVERIFIED",
        "QQ executable, selected build metadata or native payload could not be verified safely; no default build was assumed.",
        3,
    )
}

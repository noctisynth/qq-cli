//! Verify immutable archive bytes before expanding them into an owned private directory.
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::{NamedTempFile, TempDir};
use zip::ZipArchive;

use crate::error::{AppError, Result};
use crate::state::{reject_links, restrict_private};

const MAX_COMPRESSED: u64 = 64 * 1024 * 1024;
const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
const MAX_MEMBER: u64 = 128 * 1024 * 1024;

pub struct Expected<'a> {
    pub bytes: u64,
    pub sha256: &'a str,
    pub files: usize,
    pub required: &'a [(&'a str, &'a str)],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Member {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

pub struct Prepared {
    pub directory: TempDir,
    pub members: Vec<Member>,
    pub expanded_bytes: u64,
}

pub fn prepare(
    mut reader: impl Read,
    expected: &Expected<'_>,
    parent: &Path,
    deadline: Instant,
) -> Result<Prepared> {
    reject_links(parent)?;
    if expected.bytes > MAX_COMPRESSED || expected.files > 4096 || expected.files == 0 {
        return Err(archive_error());
    }
    let mut snapshot = NamedTempFile::new_in(parent).map_err(|_| AppError::storage())?;
    restrict_private(snapshot.path())?;
    let (bytes, hash) = copy_hash(&mut reader, &mut snapshot, MAX_COMPRESSED, deadline)?;
    if bytes != expected.bytes || hash != expected.sha256 {
        return Err(AppError::new(
            "ARTIFACT_MISMATCH",
            "Archive length or SHA-256 does not match the pinned artifact.",
            4,
        ));
    }
    snapshot
        .as_file()
        .sync_all()
        .map_err(|_| AppError::storage())?;
    snapshot
        .seek(SeekFrom::Start(0))
        .map_err(|_| AppError::storage())?;
    let directory = tempfile::Builder::new()
        .prefix(".qq-cli-install-")
        .tempdir_in(parent)
        .map_err(|_| AppError::storage())?;
    restrict_private(directory.path())?;
    let mut archive = ZipArchive::new(snapshot).map_err(|_| archive_error())?;
    if archive.len() > 4096 {
        return Err(archive_error());
    }
    let mut seen = HashSet::new();
    let mut expanded_paths = HashMap::new();
    let mut members = Vec::new();
    let mut expanded_bytes = 0_u64;
    for index in 0..archive.len() {
        check_deadline(deadline)?;
        let mut entry = archive.by_index(index).map_err(|_| archive_error())?;
        let name = entry.name().map_err(|_| archive_error())?.into_owned();
        let relative = member_path(&name)?;
        // Include implicit parent directories in the filesystem node budget and alias check.
        for prefix in relative
            .ancestors()
            .filter(|path| !path.as_os_str().is_empty())
        {
            let spelling = prefix.to_string_lossy().into_owned();
            let key = spelling.to_lowercase();
            if expanded_paths
                .get(&key)
                .is_some_and(|previous| previous != &spelling)
            {
                return Err(archive_error());
            }
            expanded_paths.insert(key, spelling);
            if expanded_paths.len() > 4096 {
                return Err(archive_error());
            }
        }
        if !seen.insert(relative.to_string_lossy().to_lowercase())
            || entry.encrypted()
            || entry.is_symlink()
        {
            return Err(archive_error());
        }
        let kind = entry.unix_mode().unwrap_or_default() & 0o170000;
        if kind != 0 && kind != 0o100000 && kind != 0o040000 {
            return Err(archive_error());
        }
        let target = directory.path().join(&relative);
        if entry.is_dir() {
            if entry.size() != 0 || kind == 0o100000 {
                return Err(archive_error());
            }
            std::fs::create_dir_all(target).map_err(|_| archive_error())?;
            continue;
        }
        if kind == 0o040000
            || entry.size() > MAX_MEMBER
            || expanded_bytes.saturating_add(entry.size()) > MAX_EXPANDED
        {
            return Err(archive_error());
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|_| archive_error())?;
        }
        let mut file = File::create_new(target).map_err(|_| archive_error())?;
        let declared = entry.size();
        let (actual, sha256) =
            copy_hash(&mut entry, &mut file, declared.min(MAX_MEMBER), deadline)?;
        if actual != declared {
            return Err(archive_error());
        }
        file.sync_all().map_err(|_| AppError::storage())?;
        expanded_bytes = expanded_bytes
            .checked_add(actual)
            .ok_or_else(archive_error)?;
        members.push(Member {
            path: relative.to_string_lossy().replace('\\', "/"),
            bytes: actual,
            sha256,
        });
    }
    members.sort_by(|left, right| left.path.cmp(&right.path));
    if members.len() != expected.files {
        return Err(archive_error());
    }
    for (path, hash) in expected.required {
        if !members
            .iter()
            .any(|member| member.path == *path && member.sha256 == *hash)
        {
            return Err(AppError::new(
                "ARTIFACT_INCOMPLETE",
                "Archive is missing a pinned required member or its digest differs.",
                4,
            ));
        }
    }
    Ok(Prepared {
        directory,
        members,
        expanded_bytes,
    })
}

pub fn member_path(name: &str) -> Result<PathBuf> {
    if name.len() > 512
        || name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
    {
        return Err(archive_error());
    }
    let name = name.strip_suffix('/').unwrap_or(name);
    let components = name.split('/').collect::<Vec<_>>();
    if components.len() > 32 {
        return Err(archive_error());
    }
    for component in &components {
        let stem = component
            .split('.')
            .next()
            .unwrap_or_default()
            .to_uppercase();
        let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|suffix| {
                    matches!(
                        suffix,
                        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                    )
                })
            });
        if component.is_empty()
            || matches!(*component, "." | "..")
            || component.ends_with(['.', ' '])
            || component.contains([':', '<', '>', '"', '|', '?', '*'])
            || reserved
        {
            return Err(archive_error());
        }
    }
    if components
        .first()
        .is_some_and(|component| component.eq_ignore_ascii_case(".qq-cli-installation.json"))
    {
        return Err(archive_error());
    }
    Ok(components.iter().collect())
}

fn copy_hash(
    reader: &mut impl Read,
    writer: &mut impl Write,
    limit: u64,
    deadline: Instant,
) -> Result<(u64, String)> {
    let mut bytes = 0_u64;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        check_deadline(deadline)?;
        let count = reader.read(&mut buffer).map_err(|_| archive_error())?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .filter(|bytes| *bytes <= limit)
            .ok_or_else(archive_error)?;
        writer
            .write_all(&buffer[..count])
            .map_err(|_| AppError::storage())?;
        hash.update(&buffer[..count]);
    }
    Ok((bytes, format!("{:x}", hash.finalize())))
}

fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(AppError::new(
            "DEADLINE",
            "Artifact preparation exceeded its deadline.",
            3,
        ))
    } else {
        Ok(())
    }
}
fn archive_error() -> AppError {
    AppError::new(
        "ARCHIVE_UNSAFE",
        "Archive is invalid, exceeds its resource bounds, or contains unsafe or conflicting members.",
        4,
    )
}

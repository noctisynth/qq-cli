//! Resolve a software entry without extracting or executing ASAR content.
use super::{local_path, metadata_error};
use crate::core::error::Result;
use crate::storage::state::reject_links;
use serde_json::Value;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) fn resolve(root: &Path, main: &str) -> Result<PathBuf> {
    let relative = crate::napcat::install::archive::member_path(main.trim_start_matches("./"))
        .map_err(|_| metadata_error())?;
    let parts = relative.iter().collect::<Vec<_>>();
    let mut candidate = root.to_owned();
    for (index, part) in parts.iter().enumerate() {
        candidate.push(part);
        if part.to_str().is_some_and(|p| p.ends_with(".asar")) {
            reject_links(&candidate)?;
            local_path(&candidate)?;
            inspect(&candidate, &parts[index + 1..])?;
            return Ok(candidate);
        }
    }
    Ok(candidate)
}

fn inspect(path: &Path, members: &[&std::ffi::OsStr]) -> Result<()> {
    if members.is_empty() {
        return Err(metadata_error());
    }
    let mut file = File::open(path).map_err(|_| metadata_error())?;
    let metadata = file.metadata().map_err(|_| metadata_error())?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err(metadata_error());
    }
    let mut prefix = [0_u8; 16];
    file.read_exact(&mut prefix).map_err(|_| metadata_error())?;
    let number = |i| u32::from_le_bytes(prefix[i..i + 4].try_into().unwrap()) as u64;
    let header = number(4);
    let json = number(12);
    if number(0) != 4
        || !(8..=4 * 1024 * 1024).contains(&header)
        || number(8) + 4 != header
        || json == 0
        || json > header - 8
        || header - 8 - json > 3
        || 8 + header > metadata.len()
    {
        return Err(metadata_error());
    }
    let mut bytes = vec![0_u8; (header - 8) as usize];
    file.read_exact(&mut bytes).map_err(|_| metadata_error())?;
    if bytes[json as usize..].iter().any(|&b| b != 0) {
        return Err(metadata_error());
    }
    let tree: Value =
        serde_json::from_slice(&bytes[..json as usize]).map_err(|_| metadata_error())?;
    let mut node = &tree;
    for name in members {
        if node.get("link").is_some() || node.get("unpacked").and_then(Value::as_bool) == Some(true)
        {
            return Err(metadata_error());
        }
        node = node
            .get("files")
            .and_then(|files| files.get(name.to_str()?))
            .ok_or_else(metadata_error)?;
    }
    if node.get("link").is_some() || node.get("unpacked").is_some() || node.get("files").is_some() {
        return Err(metadata_error());
    }
    let offset = node
        .get("offset")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty() && v.len() <= 20 && v.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(metadata_error)?;
    let size = node
        .get("size")
        .and_then(Value::as_u64)
        .ok_or_else(metadata_error)?;
    if (8 + header)
        .checked_add(offset)
        .and_then(|v| v.checked_add(size))
        .is_none_or(|end| end > metadata.len())
    {
        return Err(metadata_error());
    }
    Ok(())
}

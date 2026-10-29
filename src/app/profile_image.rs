//! Snapshot a caller-selected profile image; never forward a local path or URL.
use crate::core::error::{AppError, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{fs::File, io::Read, path::Path};
pub(super) fn read(path: &Path) -> Result<(String, u64)> {
    let error = || {
        AppError::new(
            "AVATAR_INPUT",
            "Avatar must be a readable nonempty regular image file of at most 8 MiB.",
            2,
        )
    };
    let source = File::open(path).map_err(|_| error())?;
    if !source.metadata().map_err(|_| error())?.is_file() {
        return Err(error());
    }
    let mut bytes = Vec::new();
    source
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error())?;
    if bytes.is_empty() || bytes.len() > 8 * 1024 * 1024 {
        return Err(error());
    }
    let size = bytes.len() as u64;
    Ok((format!("base64://{}", STANDARD.encode(bytes)), size))
}

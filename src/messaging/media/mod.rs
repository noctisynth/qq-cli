//! File transfer primitives. Trusted cache roots must come from managed runtime ownership.
//! Backend-returned paths alone are never authority to read arbitrary local files.
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::core::error::{AppError, Result};
use crate::messaging::message::{self, MediaLocator};
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::storage::state::{reject_links, restrict_private};
use crate::transport::client::Backend;

pub const MAX_DOWNLOAD_BYTES: u64 = crate::transport::media::MAX_DOWNLOAD_BYTES;
pub const MAX_BASE64_BYTES: usize = crate::transport::media::MAX_BASE64_BYTES;

/// Decode backend bytes without trusting any returned local path or URL. The caller
/// must bind them to a freshly verified media reference before invoking this function.
pub fn write_base64(
    data: &str,
    output: &Path,
    expected_size: Option<u64>,
    deadline: Instant,
    guard: impl FnOnce() -> Result<()>,
) -> Result<Transfer> {
    if data.len() > MAX_BASE64_BYTES || expected_size.is_some_and(|size| size > MAX_DOWNLOAD_BYTES)
    {
        return Err(bytes_error());
    }
    let reader = base64::read::DecoderReader::new(
        data.as_bytes(),
        &base64::engine::general_purpose::STANDARD,
    );
    publish(reader, output, expected_size, deadline, guard)
}

/// Re-read the owning message before resolving any attachment path or URL.
pub fn verify_reference<B: Backend>(
    token: &str,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<MediaLocator> {
    verify_source(token, session, references, deadline).map(|(locator, _)| locator)
}

fn verify_source<B: Backend>(
    token: &str,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<(MediaLocator, Option<String>)> {
    if !session.config.permissions.media_download {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "The local policy does not permit media downloads.",
            4,
        ));
    }
    let locator: MediaLocator = references.get("f1", token)?;
    if !matches!(locator.kind.as_str(), "image" | "file" | "record")
        || locator.message.time_seconds.is_none()
        || locator.message.sender_id.is_none()
    {
        return Err(stale());
    }
    let (raw, current) = message::fetch_raw(&locator.message, session, references, deadline)?;
    let segments = &current.value.segments;
    let mut matched = None;
    for (index, segment) in segments.iter().enumerate() {
        if locator
            .segment_index
            .is_some_and(|expected| expected != index)
        {
            continue;
        }
        if let Some(token) = segment.media().and_then(|media| media.media_ref()) {
            let candidate: MediaLocator = references.get("f1", token)?;
            if candidate.kind == locator.kind && candidate.file_id == locator.file_id {
                if locator
                    .size
                    .is_some_and(|size| candidate.size != Some(size))
                    || locator
                        .name
                        .as_ref()
                        .is_some_and(|name| candidate.name.as_ref() != Some(name))
                    || locator
                        .source_fingerprint
                        .as_ref()
                        .is_some_and(|source| candidate.source_fingerprint.as_ref() != Some(source))
                {
                    return Err(stale());
                }
                if matched.replace(candidate).is_some() {
                    return Err(stale());
                }
            }
        }
    }
    let matched = matched.ok_or_else(stale)?;
    let url = if matched.kind == "image" {
        matched
            .segment_index
            .and_then(|index| {
                raw.get("message")?
                    .get(index)?
                    .get("data")?
                    .get("url")?
                    .as_str()
            })
            .filter(|url| {
                crate::messaging::media::source::image_identity(url).as_ref()
                    == matched.source_fingerprint.as_ref()
                    && matched.source_fingerprint.is_some()
            })
            .map(str::to_owned)
    } else {
        None
    };
    Ok((matched, url))
}

fn stale() -> AppError {
    AppError::new(
        "MEDIA_REFERENCE_STALE",
        "The attachment no longer matches its owning message; locate it again before downloading.",
        4,
    )
}

pub fn download<B: Backend>(
    token: &str,
    output: &Path,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<crate::core::output::Envelope> {
    download_with_images(
        token,
        output,
        session,
        references,
        deadline,
        &crate::messaging::media::https::Native,
    )
}

pub fn download_with_images<B: Backend>(
    token: &str,
    output: &Path,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
    images: &impl crate::messaging::media::https::Images,
) -> Result<crate::core::output::Envelope> {
    download_with_format(
        token,
        output,
        crate::core::contracts::voice::Format::Original,
        session,
        references,
        deadline,
        images,
    )
}
pub fn download_as<B: Backend>(
    token: &str,
    output: &Path,
    format: crate::core::contracts::voice::Format,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<crate::core::output::Envelope> {
    download_with_format(
        token,
        output,
        format,
        session,
        references,
        deadline,
        &crate::messaging::media::https::Native,
    )
}
fn download_with_format<B: Backend>(
    token: &str,
    output: &Path,
    format: crate::core::contracts::voice::Format,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
    images: &impl crate::messaging::media::https::Images,
) -> Result<crate::core::output::Envelope> {
    crate::core::deadline::check(deadline)?;
    output_path(output)?;
    let saved: MediaLocator = references.get("f1", token)?;
    if format != crate::core::contracts::voice::Format::Original && saved.kind != "record" {
        return Err(AppError::new(
            "MEDIA_FORMAT_INVALID",
            "Format conversion requires a record attachment.",
            2,
        ));
    }
    let (locator, image_url) = verify_source(token, session, references, deadline)?;
    let converted =
        locator.kind == "record" && format != crate::core::contracts::voice::Format::Original;
    let expected_size = if converted { None } else { locator.size };
    let result = (|| {
        if let ("image", Some(url), Some(identity)) = (
            locator.kind.as_str(),
            image_url.as_deref(),
            locator.source_fingerprint.as_deref(),
        ) {
            if locator.size.is_some_and(|size| size > MAX_DOWNLOAD_BYTES) {
                return Err(bytes_error());
            }
            let (content, scope) = session.read_in_scope(
                Some(&locator.message.conversation),
                Some(&locator.message.scope),
                deadline,
                |_| images.open(url, identity, deadline),
            )?;
            if locator
                .size
                .zip(content.size)
                .is_some_and(|(old, new)| old != new)
            {
                return Err(changed());
            }
            let transfer = publish(
                content.reader,
                output,
                locator.size.or(content.size),
                deadline,
                || {
                    let observed = session.verify(deadline)?;
                    if observed.account != scope.account {
                        return Err(AppError::new(
                            "ACCOUNT_CHANGED",
                            "Media identity changed before output publication.",
                            4,
                        ));
                    }
                    Ok(())
                },
            )?;
            return Ok(crate::core::output::Envelope::typed(
                crate::core::contracts::write::Download {
                    media_ref: token.into(),
                    kind: crate::core::contracts::write::DownloadKind::Image,
                    size_bytes: transfer.size_bytes,
                    sha256: transfer.sha256,
                },
            )
            .scoped(scope));
        }
        if !locator.opaque_id {
            return Err(AppError::new(
                "MEDIA_BINDING_UNVERIFIED",
                "Backend provided only a filename alias; exact media source binding is not established.",
                4,
            ));
        }
        if locator.size.is_some_and(|size| size > MAX_DOWNLOAD_BYTES) {
            return Err(bytes_error());
        }
        let action = if locator.kind == "image" {
            crate::transport::client::Action::Image
        } else if locator.kind == "record" {
            crate::transport::client::Action::Record
        } else {
            crate::transport::client::Action::File
        };
        let payload = if locator.kind == "record" {
            serde_json::to_value(crate::core::contracts::voice::RecordPayload {
                file: None,
                file_id: Some(locator.file_id.clone()),
                out_format: format.wire().into(),
            })
            .map_err(|_| AppError::protocol())?
        } else {
            serde_json::json!({"file_id":locator.file_id})
        };
        let (mut bytes, scope) = session.read_in_scope(
            Some(&locator.message.conversation),
            Some(&locator.message.scope),
            deadline,
            |backend| backend.media_bytes(action, &payload, deadline),
        )?;
        if converted {
            bytes.size = None;
        }
        if expected_size
            .zip(bytes.size)
            .is_some_and(|(old, new)| old != new)
        {
            return Err(changed());
        }
        let transfer = write_base64(
            &bytes.encoded,
            output,
            expected_size.or(bytes.size),
            deadline,
            || {
                let observed = session.verify(deadline)?;
                if observed.account != scope.account {
                    return Err(AppError::new(
                        "ACCOUNT_CHANGED",
                        "Media identity changed before output publication.",
                        4,
                    ));
                }
                Ok(())
            },
        )?;
        Ok(
            crate::core::output::Envelope::typed(crate::core::contracts::write::Download {
                media_ref: token.into(),
                kind: if locator.kind == "image" {
                    crate::core::contracts::write::DownloadKind::Image
                } else if locator.kind == "record" {
                    crate::core::contracts::write::DownloadKind::Record
                } else {
                    crate::core::contracts::write::DownloadKind::File
                },
                size_bytes: transfer.size_bytes,
                sha256: transfer.sha256,
            })
            .scoped(scope),
        )
    })();
    result.map_err(|error: AppError| error.with_scope(locator.message.scope))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub size_bytes: u64,
    pub sha256: String,
}

/// Copy a regular file from a verified managed cache root without overwriting output.
/// Does not fetch URLs, accept a backend-supplied root, or open/execute the result.
pub fn copy_from_cache(
    source: &Path,
    trusted_roots: &[PathBuf],
    output: &Path,
    expected_size: Option<u64>,
    deadline: Instant,
) -> Result<Transfer> {
    check_deadline(deadline)?;
    if trusted_roots.is_empty() || trusted_roots.len() > 16 || !source.is_absolute() {
        return Err(source_error());
    }
    safe_leaf(source)?;
    reject_links(source).map_err(|_| source_error())?;
    let source = source.canonicalize().map_err(|_| source_error())?;
    let mut allowed = false;
    for root in trusted_roots {
        if !root.is_absolute() {
            return Err(source_error());
        }
        reject_links(root).map_err(|_| source_error())?;
        let root = root.canonicalize().map_err(|_| source_error())?;
        if !root.is_dir() {
            return Err(source_error());
        }
        allowed |= source.starts_with(&root) && source != root;
    }
    if !allowed {
        return Err(source_error());
    }
    let input = File::open(&source).map_err(|_| source_error())?;
    let before = input.metadata().map_err(|_| source_error())?;
    if !before.is_file() || before.len() > MAX_DOWNLOAD_BYTES {
        return Err(source_error());
    }
    if expected_size.is_some_and(|size| size != before.len()) {
        return Err(changed());
    }
    let checker = input.try_clone().map_err(|_| source_error())?;
    publish(input, output, Some(before.len()), deadline, || {
        let after = checker.metadata().map_err(|_| source_error())?;
        if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
            return Err(changed());
        }
        reject_links(&source).map_err(|_| source_error())
    })
}

fn publish(
    mut input: impl Read,
    output: &Path,
    expected_size: Option<u64>,
    deadline: Instant,
    guard: impl FnOnce() -> Result<()>,
) -> Result<Transfer> {
    check_deadline(deadline)?;
    let output = output_path(output)?;
    let parent = output.parent().ok_or_else(output_error)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| output_error())?;
    restrict_private(temporary.path())?;
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_deadline(deadline)?;
        let count = input
            .read(&mut buffer)
            .map_err(|_| check_deadline(deadline).err().unwrap_or_else(bytes_error))?;
        check_deadline(deadline)?;
        if count == 0 {
            break;
        }
        total = total.saturating_add(count as u64);
        if expected_size.is_some_and(|size| total > size) || total > MAX_DOWNLOAD_BYTES {
            return Err(changed());
        }
        hash.update(&buffer[..count]);
        temporary
            .write_all(&buffer[..count])
            .map_err(|_| output_error())?;
    }
    if expected_size.is_some_and(|size| size != total) {
        return Err(changed());
    }
    guard()?;
    reject_links(parent).map_err(|_| output_error())?;
    check_deadline(deadline)?;
    temporary.as_file().sync_all().map_err(|_| output_error())?;
    temporary
        .persist_noclobber(&output)
        .map_err(|_| output_error())?;
    Ok(Transfer {
        size_bytes: total,
        sha256: format!("{:x}", hash.finalize()),
    })
}

fn bytes_error() -> AppError {
    AppError::new(
        "MEDIA_BYTES_INVALID",
        "Media bytes were malformed, unreadable or exceeded the bounded content limit.",
        4,
    )
}

fn output_path(path: &Path) -> Result<PathBuf> {
    safe_leaf(path).map_err(|_| output_error())?;
    reject_links(path).map_err(|_| output_error())?;
    if path.try_exists().map_err(|_| output_error())? {
        return Err(output_error());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|_| output_error())?;
    reject_links(&parent).map_err(|_| output_error())?;
    if !parent.is_dir() {
        return Err(output_error());
    }
    Ok(parent.join(path.file_name().ok_or_else(output_error)?))
}

fn safe_leaf(path: &Path) -> Result<()> {
    let leaf = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(source_error)?;
    crate::napcat::install::archive::member_path(leaf).map_err(|_| source_error())?;
    if path.to_string_lossy().starts_with("\\\\") || path.to_string_lossy().starts_with("//") {
        return Err(source_error());
    }
    Ok(())
}
fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        return Err(AppError::new(
            "DEADLINE",
            "Media transfer exceeded the operation deadline.",
            3,
        ));
    }
    Ok(())
}
fn source_error() -> AppError {
    AppError::new(
        "MEDIA_SOURCE_INVALID",
        "Media source is not an allowed bounded regular file in a verified managed cache.",
        4,
    )
}
fn output_error() -> AppError {
    AppError::new(
        "MEDIA_OUTPUT_INVALID",
        "Media output must be a new safe file in an existing local directory.",
        2,
    )
}
fn changed() -> AppError {
    AppError::new(
        "MEDIA_CHANGED",
        "Media source no longer matches its observed size or changed during copying.",
        4,
    )
}

pub mod https;
pub mod source;

//! Atomically commit NDJSON messages and their authenticated continuation together.
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::cli::args::{ExportArgs, PageArgs, QueryArgs};
use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::history;
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::storage::state::{reject_links, restrict_private};
use crate::transport::client::Backend;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const LOCK_MAGIC: &[u8] = b"qq-cli export lock v1\n";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Checkpoint {
    schema_version: u8,
    output: PathBuf,
    scope: Scope,
    run_id: String,
    conversation: Conversation,
    query: Option<String>,
    since: Option<String>,
    until: Option<String>,
    cursor: Option<String>,
    count: u64,
    digest: String,
    bytes: usize,
    complete: bool,
}

struct Snapshot {
    body: Vec<u8>,
    checkpoint: Checkpoint,
    file_digest: String,
}

fn output_path(path: &Path) -> Result<PathBuf> {
    reject_links(path)?;
    let absolute = std::path::absolute(path).map_err(|_| file_error())?;
    let parent = absolute
        .parent()
        .ok_or_else(file_error)?
        .canonicalize()
        .map_err(|_| file_error())?;
    let path = parent.join(absolute.file_name().ok_or_else(file_error)?);
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_none_or(|n| n == ".qq-cli-export.lock" || n.is_empty())
    {
        return Err(file_error());
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(file_error)?;
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
    if reserved
        || name.eq_ignore_ascii_case(".qq-cli-export.lock")
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(file_error());
    }
    reject_links(&path)?;
    Ok(path)
}

fn snapshot(path: &Path, references: &References) -> Result<Snapshot> {
    reject_links(path)?;
    let file = File::open(path).map_err(|_| file_error())?;
    if !file.metadata().map_err(|_| file_error())?.is_file() {
        return Err(file_error());
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| file_error())?;
    if bytes.len() > MAX_BYTES || bytes.last() != Some(&b'\n') {
        return Err(modified());
    }
    let footer_start = bytes[..bytes.len() - 1]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |index| index + 1);
    let footer: Value = serde_json::from_slice(&bytes[footer_start..]).map_err(|_| modified())?;
    if footer.get("type").and_then(Value::as_str) != Some("checkpoint") {
        return Err(modified());
    }
    let token = footer
        .get("reference")
        .and_then(Value::as_str)
        .ok_or_else(modified)?;
    let checkpoint: Checkpoint = references.get("e1", token)?;
    let body = &bytes[..footer_start];
    if checkpoint.schema_version != 1
        || !checkpoint.complete && checkpoint.cursor.is_none()
        || checkpoint.output != path
        || checkpoint.bytes != body.len()
        || checkpoint.digest != digest(body)
        || checkpoint.count != body.iter().filter(|b| **b == b'\n').count() as u64
    {
        return Err(modified());
    }
    Ok(Snapshot {
        body: body.to_vec(),
        checkpoint,
        file_digest: digest(&bytes),
    })
}

pub fn inferred_instance(args: &ExportArgs, references: &References) -> Result<Option<String>> {
    if !args.resume {
        return Ok(None);
    }
    Ok(Some(
        snapshot(&output_path(&args.output)?, references)?
            .checkpoint
            .scope
            .instance_id,
    ))
}

pub fn execute<B: Backend>(
    args: &ExportArgs,
    session: &Session<'_, B>,
    references: &References,
) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(args.deadline.timeout));
    execute_until(args, session, references, deadline)
}

pub fn execute_until<B: Backend>(
    args: &ExportArgs,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let path = output_path(&args.output)?;
    let _lock = lock(path.parent().ok_or_else(file_error)?, deadline)?;
    if !args.resume && path.try_exists().map_err(|_| file_error())? {
        return Err(AppError::new(
            "OUTPUT_EXISTS",
            "Export output already exists; use --resume only for a verified qq-cli export.",
            2,
        ));
    }
    let previous = if args.resume {
        Some(snapshot(&path, references)?)
    } else {
        None
    };
    if let Some(previous) = &previous {
        let checkpoint = &previous.checkpoint;
        if checkpoint.scope.instance_id != session.instance.id
            || checkpoint.run_id != session.instance.run_id
        {
            return Err(AppError::new(
                "CURSOR_STALE",
                "Export continuation belongs to another instance or backend run.",
                4,
            ));
        }
        if args
            .conversation
            .as_ref()
            .is_some_and(|v| v != &checkpoint.conversation)
            || args
                .query
                .as_ref()
                .is_some_and(|v| Some(v) != checkpoint.query.as_ref())
            || args
                .since
                .as_ref()
                .is_some_and(|v| Some(v) != checkpoint.since.as_ref())
            || args
                .until
                .as_ref()
                .is_some_and(|v| Some(v) != checkpoint.until.as_ref())
        {
            return Err(AppError::new(
                "CURSOR_CONFLICT",
                "Explicit export arguments conflict with its checkpoint.",
                2,
            ));
        }
        session.read_in_scope(
            Some(&checkpoint.conversation),
            Some(&checkpoint.scope),
            deadline,
            |_| Ok(()),
        )?;
        if checkpoint.complete {
            return Ok(Envelope::typed(summary(&path, checkpoint.count, 0, true))
                .scoped(checkpoint.scope.clone()));
        }
    }
    let checkpoint = previous.as_ref().map(|s| &s.checkpoint);
    let conversation = args
        .conversation
        .clone()
        .or_else(|| checkpoint.map(|c| c.conversation.clone()))
        .ok_or_else(AppError::invalid)?;
    let query = args
        .query
        .clone()
        .or_else(|| checkpoint.and_then(|c| c.query.clone()));
    let since = args
        .since
        .clone()
        .or_else(|| checkpoint.and_then(|c| c.since.clone()));
    let until = args
        .until
        .clone()
        .or_else(|| checkpoint.and_then(|c| c.until.clone()));
    let request = QueryArgs {
        conversation: Some(conversation.clone()),
        since: since.clone(),
        until: until.clone(),
        page: PageArgs {
            limit: args.limit,
            cursor: checkpoint.and_then(|c| c.cursor.clone()),
            deadline: crate::cli::args::DeadlineArgs {
                timeout: args.deadline.timeout,
            },
        },
    };
    let page = history::query(
        &request,
        query.as_deref(),
        query.is_some(),
        session,
        references,
        deadline,
    )?;
    let mut body = previous
        .as_ref()
        .map(|s| s.body.clone())
        .unwrap_or_default();
    for message in &page.messages {
        serde_json::to_writer(&mut body, &json!({"type":"message","message":message}))
            .map_err(|_| file_error())?;
        body.push(b'\n');
        if body.len() > MAX_BYTES - 1024 {
            return Err(AppError::new(
                "EXPORT_LIMIT",
                "Export exceeds the 64MiB cumulative file limit; the existing output was preserved.",
                4,
            ));
        }
    }
    let complete = page.info.complete();
    let count = checkpoint.map_or(0, |c| c.count) + page.messages.len() as u64;
    let checkpoint = Checkpoint {
        schema_version: 1,
        output: path.clone(),
        scope: page.scope.clone(),
        run_id: session.instance.run_id.clone(),
        conversation,
        query,
        since,
        until,
        cursor: page.info.cursor().map(str::to_owned),
        count,
        digest: digest(&body),
        bytes: body.len(),
        complete,
    };
    let token = references.issue("e1", &checkpoint)?;
    let mut temporary =
        NamedTempFile::new_in(path.parent().ok_or_else(file_error)?).map_err(|_| file_error())?;
    restrict_private(temporary.path())?;
    temporary.write_all(&body).map_err(|_| file_error())?;
    serde_json::to_writer(
        &mut temporary,
        &json!({"type":"checkpoint","reference":token}),
    )
    .map_err(|_| file_error())?;
    temporary
        .write_all(b"\n")
        .and_then(|_| temporary.as_file().sync_all())
        .map_err(|_| file_error())?;
    if Instant::now() >= deadline {
        return Err(AppError::new(
            "DEADLINE",
            "Export deadline expired before atomic commit; output was preserved.",
            3,
        ));
    }
    if let Some(previous) = &previous {
        if snapshot(&path, references)?.file_digest != previous.file_digest {
            return Err(modified());
        }
        temporary.persist(&path).map_err(|_| file_error())?;
    } else {
        temporary
            .persist_noclobber(&path)
            .map_err(|_| file_error())?;
    }
    let summary = summary(&path, count, page.messages.len(), complete);
    Ok(Envelope::history(
        summary,
        page.scope,
        page.info,
        page.partial,
    ))
}

fn lock(parent: &Path, deadline: Instant) -> Result<File> {
    let path = parent.join(".qq-cli-export.lock");
    reject_links(&path)?;
    if !path.try_exists().map_err(|_| file_error())? {
        let mut marker = NamedTempFile::new_in(parent).map_err(|_| file_error())?;
        marker
            .write_all(LOCK_MAGIC)
            .and_then(|_| marker.as_file().sync_all())
            .map_err(|_| file_error())?;
        match marker.persist_noclobber(&path) {
            Ok(_) => (),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(file_error()),
        }
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| file_error())?;
    loop {
        match file.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(TryLockError::WouldBlock) => {
                return Err(AppError::new(
                    "EXPORT_BUSY",
                    "Another export is using this output directory.",
                    6,
                )
                .safe_to_retry());
            }
            Err(_) => return Err(file_error()),
        }
    }
    let mut marker = Vec::new();
    (&mut file)
        .take(128)
        .read_to_end(&mut marker)
        .map_err(|_| file_error())?;
    if marker != LOCK_MAGIC {
        return Err(AppError::new(
            "OUTPUT_CONFLICT",
            "Reserved export lock file has unexpected contents; it was not overwritten.",
            2,
        ));
    }
    Ok(file)
}

fn summary(
    path: &Path,
    total: u64,
    added: usize,
    complete: bool,
) -> crate::core::contracts::history::Export {
    crate::core::contracts::history::Export {
        output: path.into(),
        count: total,
        added,
        complete,
        format: crate::core::contracts::history::Format::NdjsonV1,
    }
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn modified() -> AppError {
    AppError::new(
        "EXPORT_MODIFIED",
        "Export contents or checkpoint do not match; no output was overwritten.",
        4,
    )
}
fn file_error() -> AppError {
    AppError::new(
        "EXPORT_IO",
        "Export path or file could not be accessed safely.",
        6,
    )
}

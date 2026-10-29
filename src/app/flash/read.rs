//! Project only reviewed file-set fields; local paths and arbitrary native objects stay private.
use crate::{
    cli::flash::FlashCommand,
    core::{
        contracts::{flash::*, scalar::No},
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    transport::client::{Backend, parse_raw_envelope},
};
use serde_json::Value;
use std::time::Instant;
fn text(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(AppError::protocol)
}
fn number(value: &Value, key: &str) -> Result<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite() && *n >= 0.0)
        .ok_or_else(AppError::protocol)
}
fn counts(value: &Value) -> Result<Counts> {
    Ok(Counts {
        succeeded: number(value, "successCount")?,
        failed: number(value, "failedCount")?,
    })
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(AppError::protocol)
}
pub(super) fn execute<B: Backend>(
    command: &FlashCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Envelope> {
    if !session.config.permissions.targets.is_empty() {
        return Err(AppError::new(
            "API_TARGET_SCOPE_UNKNOWN",
            "File-set metadata cannot prove original conversation scopes.",
            4,
        ));
    }
    let (action, payload, set) = match command {
        FlashCommand::Resolve { share_code, .. } => (
            "get_fileset_id",
            serde_json::to_value(ResolvePayload {
                share_code: share_code.to_string(),
            }),
            None,
        ),
        FlashCommand::Info { fileset_id, .. }
        | FlashCommand::List { fileset_id, .. }
        | FlashCommand::Link { fileset_id, .. } => {
            let action = match command {
                FlashCommand::Info { .. } => "get_fileset_info",
                FlashCommand::List { .. } => "get_flash_file_list",
                _ => "get_share_link",
            };
            (
                action,
                serde_json::to_value(SetPayload {
                    fileset_id: fileset_id.to_string(),
                }),
                Some(fileset_id.to_string()),
            )
        }
        FlashCommand::Url {
            fileset_id,
            file_name,
            file_index,
            ..
        } => {
            if !session.config.permissions.media_download {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "The local policy does not permit file URL retrieval.",
                    4,
                ));
            }
            if file_name.is_none() == file_index.is_none() {
                return Err(AppError::invalid());
            }
            (
                "get_flash_file_url",
                serde_json::to_value(UrlPayload {
                    fileset_id: fileset_id.to_string(),
                    file_name: file_name.as_ref().map(ToString::to_string),
                    file_index: file_index.map(f64::from),
                }),
                Some(fileset_id.to_string()),
            )
        }
        _ => return Err(AppError::invalid()),
    };
    let payload = payload.map_err(|_| AppError::invalid())?;
    let (data, scope) = session.read(None, deadline, |backend| {
        parse_raw_envelope(backend.call_raw(
            &action.parse().map_err(|_| AppError::protocol())?,
            &payload,
            deadline,
        )?)
    })?;
    if set.is_some() && data.get("result").and_then(Value::as_i64) != Some(0) {
        return Err(AppError::new(
            "FLASH_NATIVE_REJECTED",
            "Native file-set service did not report success.",
            4,
        )
        .with_scope(scope));
    }
    let result = match command {
        FlashCommand::Resolve { .. } => Envelope::typed(Resolved {
            fileset_id: text(&data, "fileset_id")?,
            verification_available: No,
        }),
        FlashCommand::Info { .. } => {
            let set = set.ok_or_else(AppError::protocol)?;
            let file = data.get("fileSet").ok_or_else(AppError::protocol)?;
            if text(file, "fileSetId")? != set {
                return Err(AppError::protocol());
            }
            Envelope::typed(Info {
                fileset_id: set,
                name: file
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(AppError::protocol)?,
                native_file_count: number(file, "totalFileCount")?,
                native_total_size: number(file, "totalFileSize")?,
                native_status: number(file, "status")?,
                native_upload_status: number(file, "uploadStatus")?,
                native_download_status: number(file, "downloadStatus")?,
                upload: counts(file.get("uploadInfo").ok_or_else(AppError::protocol)?)?,
                download: counts(file.get("downloadInfo").ok_or_else(AppError::protocol)?)?,
                transfer_verified: No,
            })
        }
        FlashCommand::List { limit, .. } => {
            let set = set.ok_or_else(AppError::protocol)?;
            let batches = array(&data, "fileLists")?;
            let mut count = 0_usize;
            let mut items = Vec::new();
            if batches.len() > session.config.scan_limit {
                return Err(AppError::new(
                    "SCAN_LIMIT",
                    "Native file-set batches exceed the local scan budget.",
                    3,
                ));
            }
            for batch in batches {
                let rows = array(batch, "fileList")?;
                count = count.saturating_add(rows.len());
                if count > session.config.scan_limit {
                    return Err(AppError::new(
                        "SCAN_LIMIT",
                        "Native file-set files exceed the local scan budget.",
                        3,
                    ));
                }
                for (index, row) in rows.iter().enumerate() {
                    crate::core::deadline::check(deadline)?;
                    if items.len() >= usize::from(limit.get()) {
                        continue;
                    }
                    if text(row, "fileSetId")? != set {
                        return Err(AppError::protocol());
                    }
                    let native_size = text(row, "fileSize")?;
                    if native_size.len() > 32 || !native_size.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(AppError::protocol());
                    }
                    items.push(File {
                        client_id: text(row, "cliFileId")?,
                        name: text(row, "name")?,
                        directory: row
                            .get("isDir")
                            .and_then(Value::as_bool)
                            .ok_or_else(AppError::protocol)?,
                        native_size,
                        batch_index: index,
                        native_client_index: number(row, "cliFileIndex")?,
                        native_upload_status: number(row, "uploadStatus")?,
                        native_download_status: number(row, "downloadStatus")?,
                    });
                }
            }
            Envelope::typed(Listing {
                fileset_id: set,
                items,
                complete: No,
                truncated: count > usize::from(limit.get()),
            })
        }
        _ => {
            let key = if matches!(command, FlashCommand::Link { .. }) {
                "shareLink"
            } else {
                "transferUrl"
            };
            let url = data
                .get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
            Envelope::typed(Link {
                fileset_id: set.ok_or_else(AppError::protocol)?,
                url_available: url.is_some(),
                url,
                native_expiry: data
                    .get("expireTimestamp")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        }
    };
    Ok(result.scoped(scope))
}

//! Group batches and private URLs are projected only inside their actual source scope.
use crate::{
    cli::files::FileCommand,
    core::{
        contracts::{account::NumberOrString, files::*, scalar::No},
        domain::{Conversation, identifier, is_exact_js_id},
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{media, session::Session},
    storage::references::References,
    transport::client::{Backend, parse_raw_envelope},
};
use serde_json::Value;
use std::time::Instant;
fn text(row: &Value, key: &str) -> Result<String> {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(AppError::protocol)
}
fn number(row: &Value, key: &str) -> Result<f64> {
    row.get(key)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite() && *v >= 0.0)
        .ok_or_else(AppError::protocol)
}
fn array<'a>(row: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    row.get(key)
        .and_then(Value::as_array)
        .ok_or_else(AppError::protocol)
}
fn group_matches(row: &Value, target: &Conversation) -> Result<()> {
    if row.get("group_id").and_then(identifier).as_deref() != Some(target.id()) {
        return Err(AppError::protocol());
    }
    Ok(())
}
pub(super) fn execute<B: Backend>(
    command: &FileCommand,
    session: &Session<'_, B>,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    let (action, payload, target, expected) = match command {
        FileCommand::Info { group_id, .. } => (
            "get_group_file_system_info",
            serde_json::to_value(GroupPayload {
                group_id: group_id.to_string(),
            }),
            command.target().ok_or_else(AppError::invalid)?,
            None,
        ),
        FileCommand::List {
            group_id,
            folder,
            limit,
            ..
        } => {
            let count = NumberOrString::Number(f64::from(limit.get()));
            if let Some(folder) = folder {
                (
                    "get_group_files_by_folder",
                    serde_json::to_value(FolderListPayload {
                        folder: FolderPayload {
                            group_id: group_id.to_string(),
                            folder: Some(folder.to_string()),
                            folder_id: None,
                        },
                        file_count: count,
                    }),
                    command.target().ok_or_else(AppError::invalid)?,
                    None,
                )
            } else {
                (
                    "get_group_root_files",
                    serde_json::to_value(RootPayload {
                        group_id: group_id.to_string(),
                        file_count: count,
                    }),
                    command.target().ok_or_else(AppError::invalid)?,
                    None,
                )
            }
        }
        FileCommand::Url {
            group_id, file_id, ..
        } => {
            if !session.config.permissions.media_download {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "The local policy does not permit file URL retrieval.",
                    4,
                ));
            }
            if !is_exact_js_id(group_id) {
                return Err(AppError::invalid());
            }
            (
                "get_group_file_url",
                serde_json::to_value(GroupFilePayload {
                    group_id: group_id.to_string(),
                    file_id: file_id.to_string(),
                }),
                command.target().ok_or_else(AppError::invalid)?,
                None,
            )
        }
        FileCommand::PrivateUrl { media_ref, .. } => {
            let locator = media::verify_reference(
                media_ref,
                session,
                references.ok_or_else(AppError::storage)?,
                deadline,
            )?;
            if locator.kind != "file"
                || !locator.opaque_id
                || locator.message.conversation.is_group()
            {
                return Err(AppError::invalid());
            }
            (
                "get_private_file_url",
                serde_json::to_value(PrivateUrlPayload {
                    file_id: locator.file_id,
                }),
                locator.message.conversation,
                Some(locator.message.scope),
            )
        }
        _ => return Err(AppError::invalid()),
    };
    let payload = payload.map_err(|_| AppError::invalid())?;
    let (data, scope) =
        session.read_in_scope(Some(&target), expected.as_ref(), deadline, |backend| {
            parse_raw_envelope(backend.call_raw(
                &action.parse().map_err(|_| AppError::protocol())?,
                &payload,
                deadline,
            )?)
        })?;
    let result = match command {
        FileCommand::Info { .. } => Envelope::typed(Info {
            conversation: target,
            native_file_count: number(&data, "file_count")?,
            native_limit_count: number(&data, "limit_count")?,
            native_used_space: number(&data, "used_space")?,
            native_total_space: number(&data, "total_space")?,
            quota_verified: No,
        }),
        FileCommand::List { folder, limit, .. } => {
            let files = array(&data, "files")?;
            let folders = array(&data, "folders")?;
            if files.len().saturating_add(folders.len()) > session.config.scan_limit {
                return Err(AppError::new(
                    "SCAN_LIMIT",
                    "Group file batch exceeds the local scan budget.",
                    3,
                ));
            }
            let count = usize::from(limit.get());
            let truncated = files.len() + folders.len() > count;
            let files = files
                .iter()
                .take(count)
                .map(|row| {
                    group_matches(row, &target)?;
                    Ok(File {
                        file_id: text(row, "file_id")?,
                        name: text(row, "file_name")?,
                        native_size: number(row, "file_size")?,
                        uploader_id: row
                            .get("uploader")
                            .and_then(identifier)
                            .filter(|v| is_exact_js_id(v)),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let folders = folders
                .iter()
                .take(count.saturating_sub(files.len()))
                .map(|row| {
                    group_matches(row, &target)?;
                    Ok(Folder {
                        folder_id: text(row, "folder_id")?,
                        name: text(row, "folder_name")?,
                        native_file_count: row
                            .get("total_file_count")
                            .map(|_| number(row, "total_file_count"))
                            .transpose()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Listing {
                conversation: target,
                folder_id: folder.as_ref().map(ToString::to_string),
                files,
                folders,
                complete: No,
                truncated,
            })
        }
        _ => {
            let url = data
                .get("url")
                .map(|v| v.as_str().map(str::to_owned).ok_or_else(AppError::protocol))
                .transpose()?
                .filter(|s| !s.is_empty());
            Envelope::typed(Url {
                conversation: target,
                url_available: url.is_some(),
                url,
            })
        }
    };
    Ok(result.scoped(scope))
}

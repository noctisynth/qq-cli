//! Native list failures and unsupported media shapes are never normalized into empty success.
use crate::{
    cli::album::AlbumCommand,
    core::{
        contracts::{album::*, scalar::No},
        domain::Conversation,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    transport::client::{Backend, parse_raw_envelope},
};
use serde_json::Value;
use std::time::Instant;
fn required(row: &Value, key: &str) -> Result<String> {
    row.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(AppError::protocol)
}
pub(super) fn execute<B: Backend>(
    command: &AlbumCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Envelope> {
    let group = command.group();
    let (action, payload, limit) = match command {
        AlbumCommand::List {
            attach_info, limit, ..
        } => (
            "get_qun_album_list",
            serde_json::to_value(ListPayload {
                group_id: group.to_string(),
                attach_info: attach_info.as_ref().map(ToString::to_string),
            }),
            *limit,
        ),
        AlbumCommand::Media {
            album_id,
            attach_info,
            limit,
            ..
        } => (
            "get_group_album_media_list",
            serde_json::to_value(MediaPayload {
                group_id: group.to_string(),
                album_id: album_id.to_string(),
                attach_info: attach_info.as_ref().map(ToString::to_string),
            }),
            *limit,
        ),
        _ => return Err(AppError::invalid()),
    };
    let target = Conversation::Group(group.to_string());
    let payload = payload.map_err(|_| AppError::invalid())?;
    let (data, scope) = session.read(Some(&target), deadline, |backend| {
        parse_raw_envelope(backend.call_raw(
            &action.parse().map_err(|_| AppError::protocol())?,
            &payload,
            deadline,
        )?)
    })?;
    if data.get("result").is_some_and(|v| v.as_i64() != Some(0)) {
        return Err(AppError::new(
            "ALBUM_NATIVE_REJECTED",
            "Native album service did not report success.",
            4,
        )
        .with_scope(scope));
    }
    let key = if matches!(command, AlbumCommand::List { .. }) {
        "album_list"
    } else {
        "media_list"
    };
    let rows=data.get(key).and_then(Value::as_array).ok_or_else(||AppError::new("ALBUM_SCHEMA_UNSUPPORTED","Native album page does not expose the reviewed list shape; inspect it with an explicit API call.",3))?;
    if rows.len() > session.config.scan_limit {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Native album page exceeds the local scan budget.",
            3,
        ));
    }
    let truncated = rows.len() > usize::from(limit.get());
    let next = if truncated {
        None
    } else {
        data.get("attach_info")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let output = match command {
        AlbumCommand::List { .. } => {
            let items = rows
                .iter()
                .take(usize::from(limit.get()))
                .map(|row| {
                    Ok(Album {
                        album_id: required(row, "album_id")?,
                        name: row
                            .get("name")
                            .or_else(|| row.get("album_name"))
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Albums {
                group_id: group.clone(),
                items,
                complete: No,
                truncated,
                native_has_more: data
                    .get("has_more")
                    .and_then(Value::as_bool)
                    .ok_or_else(AppError::protocol)?,
                next_attach_info: next,
            })
        }
        AlbumCommand::Media { album_id, .. } => {
            let items = rows
                .iter()
                .take(usize::from(limit.get()))
                .map(|row| {
                    Ok(Media {
                        media_id: required(row, "media_id")?,
                        url: row.get("url").and_then(Value::as_str).map(str::to_owned),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Medias {
                group_id: group.clone(),
                album_id: album_id.to_string(),
                items,
                complete: No,
                truncated,
                next_attach_info: next,
            })
        }
        _ => return Err(AppError::invalid()),
    };
    Ok(output.scoped(scope))
}

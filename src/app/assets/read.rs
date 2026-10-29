//! Repository snapshots preserve only reviewed fields, never native local paths or bodies.
use crate::{
    cli::{
        args::Command,
        assets::{EmojiCommand, FavoriteCommand},
    },
    core::{
        contracts::{account::NumberOrString, assets::*, scalar::No},
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    transport::client::{Backend, parse_raw_envelope},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::time::Instant;
fn opt<T: DeserializeOwned>(value: &Value, key: &str) -> Result<Option<T>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|_| AppError::protocol()),
    }
}
fn required<T: DeserializeOwned>(value: &Value, key: &str) -> Result<T> {
    opt(value, key)?.ok_or_else(AppError::protocol)
}
fn encode(value: impl Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| AppError::invalid())
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or_else(AppError::protocol)
}
pub(super) fn execute<B: Backend>(
    command: &Command,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let (action, payload, limit) = match command {
        Command::Favorite {
            command: FavoriteCommand::List {
                category, limit, ..
            },
        } => {
            if !session.config.permissions.targets.is_empty() {
                return Err(AppError::new(
                    "ASSET_SCOPE_UNKNOWN",
                    "Favorite summaries cannot prove their original conversations under the local target restriction.",
                    4,
                ));
            }
            (
                "get_collection_list",
                encode(FavoritesPayload {
                    category: category.to_string(),
                    count: usize::from(limit.get())
                        .min(session.config.scan_limit)
                        .to_string(),
                })?,
                limit,
            )
        }
        Command::Emoji {
            command: EmojiCommand::List { limit, .. },
        } => (
            "fetch_custom_face_detail",
            encode(EmojiPayload {
                count: NumberOrString::Number(
                    f64::from(limit.get()).min(session.config.scan_limit as f64),
                ),
            })?,
            limit,
        ),
        _ => return Ok(None),
    };
    let action = action.parse().map_err(|_| AppError::protocol())?;
    let (value, scope) = session.read(None, deadline, |backend| {
        parse_raw_envelope(backend.call_raw(&action, &payload, deadline)?)
    })?;
    let count = usize::from(limit.get()).min(session.config.scan_limit);
    let result = match command {
        Command::Favorite { .. } => {
            let codes = [
                opt::<i64>(&value, "result")?,
                opt::<i64>(&value, "errCode")?,
            ];
            if codes.iter().all(Option::is_none) {
                return Err(AppError::protocol());
            }
            if codes.iter().flatten().any(|code| *code != 0) {
                return Err(AppError::new(
                    "BACKEND_REJECTED",
                    "Backend rejected the favorite query.",
                    4,
                ));
            }
            let search = value
                .get("collectionSearchList")
                .ok_or_else(AppError::protocol)?;
            let rows = array(
                search
                    .get("collectionItemList")
                    .ok_or_else(AppError::protocol)?,
            )?;
            if rows.len() > session.config.scan_limit {
                return Err(AppError::new(
                    "SCAN_LIMIT",
                    "Favorite snapshot exceeds the configured scan limit.",
                    4,
                ));
            }
            let items = rows
                .iter()
                .take(count)
                .map(|row| {
                    let summary = row
                        .get("summary")
                        .and_then(|v| v.get("richMediaSummary"))
                        .filter(|v| v.is_object());
                    Ok(Favorite {
                        id: required(row, "cid")?,
                        kind: required(row, "type")?,
                        status: required(row, "status")?,
                        category: required(row, "category")?,
                        create_time: required(row, "createTime")?,
                        collect_time: required(row, "collectTime")?,
                        modify_time: required(row, "modifyTime")?,
                        title: summary.map(|v| opt(v, "title")).transpose()?.flatten(),
                        brief: summary.map(|v| opt(v, "brief")).transpose()?.flatten(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Favorites {
                complete: No,
                body_available: No,
                has_more: required(search, "hasMore")?,
                items,
            })
        }
        Command::Emoji { .. } => {
            let rows = array(&value)?;
            if rows.len() > session.config.scan_limit {
                return Err(AppError::new(
                    "SCAN_LIMIT",
                    "Emoji snapshot exceeds the configured scan limit.",
                    4,
                ));
            }
            let items = rows
                .iter()
                .take(count)
                .map(|row| {
                    let emoji_id = opt::<i64>(row, "emoId")?.filter(|id| {
                        crate::core::domain::parse_js_integer(&id.to_string()).is_some()
                    });
                    Ok(Emoji {
                        res_id: required(row, "resId")?,
                        emoji_id,
                        md5: opt(row, "md5")?,
                        description: opt(row, "desc")?,
                        url: opt(row, "url")?,
                        native_exists: opt(row, "isExist")?,
                        animated_png: opt(row, "isAPNG")?,
                        market_face: opt(row, "isMarkFace")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Emojis {
                complete: No,
                items,
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(result.scoped(scope)))
}

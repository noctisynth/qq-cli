//! Only reviewed online-file metadata leaves the native service boundary.
use crate::{
    cli::online::OnlineCommand,
    core::{
        contracts::{online::*, scalar::No},
        domain::Conversation,
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
pub(super) fn execute<B: Backend>(
    command: &OnlineCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Envelope> {
    let OnlineCommand::List { user_id, limit, .. } = command else {
        return Err(AppError::invalid());
    };
    let target = Conversation::Private(user_id.to_string());
    let payload = serde_json::to_value(UserPayload {
        user_id: user_id.to_string(),
    })
    .map_err(|_| AppError::invalid())?;
    let (data, scope) = session.read(Some(&target), deadline, |backend| {
        parse_raw_envelope(
            backend.call_raw(
                &"get_online_file_msg"
                    .parse()
                    .map_err(|_| AppError::protocol())?,
                &payload,
                deadline,
            )?,
        )
    })?;
    if data.get("result").and_then(Value::as_i64) != Some(0) {
        return Err(AppError::new(
            "ONLINE_NATIVE_REJECTED",
            "Native online-file listing did not report success.",
            4,
        )
        .with_scope(scope));
    }
    let messages = data
        .get("msgList")
        .and_then(Value::as_array)
        .ok_or_else(AppError::protocol)?;
    let mut count = messages.len();
    let mut items = Vec::new();
    let mut truncated = false;
    if count > session.config.scan_limit {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Online-file messages exceed the scan budget.",
            3,
        ));
    }
    for msg in messages {
        crate::core::deadline::check(deadline)?;
        let elements = msg
            .get("elements")
            .and_then(Value::as_array)
            .ok_or_else(AppError::protocol)?;
        count = count.saturating_add(elements.len());
        if count > session.config.scan_limit {
            return Err(AppError::new(
                "SCAN_LIMIT",
                "Online-file elements exceed the scan budget.",
                3,
            ));
        }
        for el in elements {
            let kind = match el.get("elementType").and_then(Value::as_u64) {
                Some(23) => Kind::File,
                Some(30) => Kind::Directory,
                _ => continue,
            };
            if items.len() >= usize::from(limit.get()) {
                truncated = true;
                continue;
            }
            let file = el.get("fileElement").ok_or_else(AppError::protocol)?;
            let size = file
                .get("fileSize")
                .and_then(Value::as_str)
                .ok_or_else(AppError::protocol)?;
            if size.len() > 32 || !size.bytes().all(|b| b.is_ascii_digit()) {
                return Err(AppError::protocol());
            }
            items.push(Item {
                native_msg_id: text(msg, "msgId")?,
                element_id: text(el, "elementId")?,
                native_time: text(msg, "msgTime")?,
                kind,
                name: text(file, "fileName")?,
                native_size: size.to_owned(),
            });
        }
    }
    Ok(Envelope::typed(Listing {
        conversation: target,
        items,
        complete: No,
        truncated,
    })
    .scoped(scope))
}

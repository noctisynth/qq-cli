//! One bounded upstream reaction listing after a fresh source check.
use crate::{
    cli::args::MessageCommand,
    core::{
        contracts::{interaction::*, scalar::No},
        domain::Conversation,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{message, session::Session},
    storage::references::References,
    transport::client::{Backend, parse_raw_envelope},
};
use std::time::Instant;
pub(super) fn execute<B: Backend>(
    command: &MessageCommand,
    session: &Session<'_, B>,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let MessageCommand::Reactions {
        message_ref,
        emoji_id,
        emoji_type,
        limit,
        ..
    } = command
    else {
        return Ok(None);
    };
    let refs = references.ok_or_else(AppError::storage)?;
    let locator = message::load_locator(refs, message_ref)?;
    let group_id = match &locator.conversation {
        Conversation::Group(id) => Some(id.clone()),
        Conversation::Private(_) => None,
    };
    if group_id.is_none() && locator.backend_id.parse::<i32>().is_err() {
        return Err(AppError::new(
            "MESSAGE_SHORT_ID_REQUIRED",
            "Private reaction listings require an upstream cached 32-bit message ID.",
            4,
        ));
    }
    message::fetch(&locator, session, refs, deadline)?;
    let count = usize::from(limit.get()).min(session.config.scan_limit);
    let payload = serde_json::to_value(ListPayload {
        group_id,
        message_id: locator.backend_id,
        emoji_id: emoji_id.to_string(),
        emoji_type: emoji_type.as_ref().map(ToString::to_string),
        count: count as f64,
    })
    .map_err(|_| AppError::protocol())?;
    let action = "get_emoji_likes"
        .parse()
        .map_err(|_| AppError::protocol())?;
    let (data, scope) = session.read_in_scope(
        Some(&locator.conversation),
        Some(&locator.scope),
        deadline,
        |backend| parse_raw_envelope(backend.call_raw(&action, &payload, deadline)?),
    )?;
    let rows = data
        .get("emoji_like_list")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(AppError::protocol)?;
    if rows.len() > session.config.scan_limit {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Reaction snapshot exceeds the configured scan limit.",
            4,
        ));
    }
    let items = rows
        .iter()
        .take(count)
        .map(|row| {
            Ok(Reactor {
                reactor_id: row
                    .get("user_id")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(AppError::protocol)?
                    .into(),
                nickname: row
                    .get("nick_name")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(AppError::protocol)?
                    .into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(
        Envelope::typed(Reactions {
            conversation: locator.conversation,
            emoji_id: emoji_id.to_string(),
            complete: No,
            items,
        })
        .scoped(scope),
    ))
}

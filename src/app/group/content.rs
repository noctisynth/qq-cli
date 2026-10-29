//! Announcements and web-derived essences never mint original-message authority.
use super::read::{array, budget, id, opt, required};
use crate::{
    cli::group::GroupCommand,
    core::{
        contracts::{group_content::*, scalar::No},
        domain::Conversation,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{message, session::Session},
    transport::client::{Backend, parse_raw_envelope},
};
use serde_json::Value;
use std::time::Instant;
fn integer(value: &Value, key: &str) -> Result<String> {
    let raw = value.get(key).ok_or_else(AppError::protocol)?;
    let text = raw
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| raw.to_string());
    crate::core::domain::parse_js_integer(&text)
        .map(|v| v.to_string())
        .ok_or_else(AppError::protocol)
}
pub(super) fn execute<B: Backend>(
    command: &GroupCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let (action, group, limit) = match command {
        GroupCommand::Notices {
            group_id, limit, ..
        } => ("_get_group_notice", group_id, limit),
        GroupCommand::EssenceList {
            group_id, limit, ..
        } => ("get_essence_msg_list", group_id, limit),
        _ => return Ok(None),
    };
    let target = Conversation::Group(group.to_string());
    let payload = serde_json::to_value(GroupPayload {
        group_id: group.to_string(),
    })
    .map_err(|_| AppError::invalid())?;
    let action = action.parse().map_err(|_| AppError::protocol())?;
    let (data, scope) = session.read(Some(&target), deadline, |backend| {
        parse_raw_envelope(backend.call_raw(&action, &payload, deadline)?)
    })?;
    let rows = array(&data)?;
    budget(rows.len(), session.config.scan_limit)?;
    let result = match command {
        GroupCommand::Notices { .. } => {
            let items = rows
                .iter()
                .take(usize::from(limit.get()))
                .map(|row| {
                    crate::core::deadline::check(deadline)?;
                    let message = row.get("message").ok_or_else(AppError::protocol)?;
                    let images = array(
                        message
                            .get("images")
                            .or_else(|| message.get("image"))
                            .ok_or_else(AppError::protocol)?,
                    )?;
                    budget(images.len(), session.config.scan_limit)?;
                    let images = images
                        .iter()
                        .map(|image| {
                            Ok(NoticeImage {
                                id: required(image, "id")?,
                                width: required(image, "width")?,
                                height: required(image, "height")?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Ok(Notice {
                        notice_id: required(row, "notice_id")?,
                        sender_id: id(row, "sender_id")?,
                        native_time: required(row, "publish_time")?,
                        text: required(message, "text")?,
                        images,
                        read_count: opt(row, "read_num")?,
                        send_to_new_member: opt(row, "send_to_new_member")?,
                        pinned: opt(row, "pinned")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Notices {
                group_id: group.clone(),
                complete: No,
                items,
            })
        }
        GroupCommand::EssenceList { .. } => {
            let items = rows
                .iter()
                .take(usize::from(limit.get()))
                .map(|row| {
                    crate::core::deadline::check(deadline)?;
                    Ok(Essence {
                        sequence: integer(row, "msg_seq")?,
                        random: integer(row, "msg_random")?,
                        native_message_id: integer(row, "message_id")?,
                        sender_id: id(row, "sender_id")?,
                        sender_nickname: required(row, "sender_nick")?,
                        operator_id: id(row, "operator_id")?,
                        operator_nickname: required(row, "operator_nick")?,
                        native_operator_time: required(row, "operator_time")?,
                        segments: message::snapshot_segments(
                            row.get("content").ok_or_else(AppError::protocol)?,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(Essences {
                group_id: group.clone(),
                complete: No,
                items,
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(result.scoped(scope)))
}

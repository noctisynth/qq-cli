//! Group snapshots project only reviewed fields and retain upstream ambiguity.
use crate::{
    cli::group::GroupCommand,
    core::{
        contracts::{
            account::NumberOrString, group::BoolOrString, group_query::*, resource::Row, scalar::No,
        },
        domain::{Conversation, identifier},
        error::{AppError, Result},
        input::DecimalId,
        output::Envelope,
    },
    messaging::{read, session::Session},
    transport::client::{Backend, parse_raw_envelope},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::time::Instant;
pub(super) fn opt<T: DeserializeOwned>(value: &Value, key: &str) -> Result<Option<T>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|_| AppError::protocol()),
    }
}
pub(super) fn required<T: DeserializeOwned>(value: &Value, key: &str) -> Result<T> {
    opt(value, key)?.ok_or_else(AppError::protocol)
}
pub(super) fn id(value: &Value, key: &str) -> Result<DecimalId> {
    value
        .get(key)
        .and_then(identifier)
        .and_then(|v| v.parse().ok())
        .ok_or_else(AppError::protocol)
}
fn encode(value: impl Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| AppError::invalid())
}
pub(super) fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or_else(AppError::protocol)
}
fn honor(value: &Value) -> Result<HonorUser> {
    if !value.is_object() {
        return Err(AppError::protocol());
    }
    Ok(HonorUser {
        user_id: value
            .get("user_id")
            .and_then(identifier)
            .and_then(|v| v.parse().ok()),
        nickname: opt(value, "nickname")?,
        avatar_url: opt(value, "avatar")?,
        description: opt(value, "description")?,
    })
}
fn honors(value: &Value, key: &str) -> Result<Vec<HonorUser>> {
    array(value.get(key).ok_or_else(AppError::protocol)?)?
        .iter()
        .map(honor)
        .collect()
}
pub(super) fn budget(size: usize, max: usize) -> Result<()> {
    if size > max {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Group snapshot exceeds the configured scanLimit.",
            4,
        ));
    }
    Ok(())
}
pub(super) fn execute<B: Backend>(
    command: &GroupCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let (action, payload, group_id) = match command {
        GroupCommand::Signed { group_id, .. } => (
            "get_group_signed_list",
            encode(crate::core::contracts::group_content::SignedPayload {
                group_id: crate::core::contracts::account::NumberOrString::Text(
                    group_id.to_string(),
                ),
            })?,
            group_id,
        ),
        GroupCommand::CardJson { group_id, .. } => (
            "ArkShareGroup",
            encode(crate::core::contracts::card::GroupPayload {
                group_id: group_id.to_string(),
            })?,
            group_id,
        ),
        GroupCommand::Member {
            group_id,
            user_id,
            cached,
            ..
        } => (
            "get_group_member_info",
            encode(MemberPayload {
                group_id: group_id.to_string(),
                user_id: user_id.to_string(),
                no_cache: Some(BoolOrString::Bool(!*cached)),
            })?,
            group_id,
        ),
        GroupCommand::Detail { group_id, .. } => (
            "get_group_detail_info",
            encode(GroupPayload {
                group_id: group_id.to_string(),
            })?,
            group_id,
        ),
        GroupCommand::Extended { group_id, .. } => (
            "get_group_info_ex",
            encode(GroupPayload {
                group_id: group_id.to_string(),
            })?,
            group_id,
        ),
        GroupCommand::Share {
            group_id,
            full,
            source_id,
            additional,
            ..
        } => (
            "get_group_share_link",
            encode(SharePayload {
                group_id: NumberOrString::Text(group_id.to_string()),
                need_short_url: Some(BoolOrString::Bool(!*full)),
                src_id: Some(NumberOrString::Number(f64::from(*source_id))),
                additional_param: additional.as_ref().map(ToString::to_string),
            })?,
            group_id,
        ),
        GroupCommand::Honors { group_id, kind, .. } => (
            "get_group_honor_info",
            encode(HonorsPayload {
                group_id: group_id.to_string(),
                kind: Some(*kind),
            })?,
            group_id,
        ),
        GroupCommand::Muted { group_id, .. } => (
            "get_group_shut_list",
            encode(GroupPayload {
                group_id: group_id.to_string(),
            })?,
            group_id,
        ),
        GroupCommand::MentionQuota { group_id, .. } => (
            "get_group_at_all_remain",
            encode(GroupPayload {
                group_id: group_id.to_string(),
            })?,
            group_id,
        ),
        _ => return Ok(None),
    };
    let target = Conversation::Group(group_id.to_string());
    let (value, scope) = session.read(Some(&target), deadline, |backend| {
        parse_raw_envelope(backend.call_raw(
            &action.parse().map_err(|_| AppError::protocol())?,
            &payload,
            deadline,
        )?)
    })?;
    let envelope = match command {
        GroupCommand::Signed { limit, .. } => {
            let rows = array(&value)?;
            budget(rows.len(), session.config.scan_limit)?;
            let users = rows
                .iter()
                .take(usize::from(limit.get()))
                .map(|row| {
                    let user_id = id(row, "user_id")?;
                    if !crate::core::domain::is_exact_js_id(&user_id) {
                        return Err(AppError::protocol());
                    }
                    Ok(crate::core::contracts::group_content::Signer {
                        user_id,
                        nickname: required(row, "nick")?,
                        native_time: required(row, "time")?,
                        rank: required(row, "rank")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Envelope::typed(crate::core::contracts::group_content::Signed {
                group_id: group_id.clone(),
                complete: No,
                users,
            })
        }
        GroupCommand::CardJson { .. } => {
            let card_json = value
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    AppError::new("CARD_UNAVAILABLE", "Backend returned no card content.", 4)
                })?
                .to_owned();
            Envelope::typed(crate::core::contracts::card::Card {
                conversation: Conversation::Group(group_id.to_string()),
                card_json,
            })
        }
        GroupCommand::Member { user_id, .. } => {
            if id(&value, "group_id")? != *group_id || id(&value, "user_id")? != *user_id {
                return Err(AppError::protocol());
            }
            let Row::Member(member) = read::member(&value)? else {
                return Err(AppError::protocol());
            };
            Envelope::typed(MemberDetails {
                group_id: group_id.clone(),
                member,
                title: opt(&value, "title")?,
                join_time: opt(&value, "join_time")?,
                last_sent_time: opt(&value, "last_sent_time")?,
                shut_up_timestamp: opt(&value, "shut_up_timestamp")?,
            })
        }
        GroupCommand::Detail { .. } => {
            if id(&value, "group_id")? != *group_id {
                return Err(AppError::protocol());
            }
            let state: i64 = required(&value, "group_all_shut")?;
            if !matches!(state, -1 | 0) {
                return Err(AppError::protocol());
            }
            Envelope::typed(Details {
                group_id: group_id.clone(),
                name: required(&value, "group_name")?,
                member_count: required(&value, "member_count")?,
                max_member_count: required(&value, "max_member_count")?,
                whole_group_muted: state == -1,
            })
        }
        GroupCommand::Extended { .. } => {
            if id(&value, "groupCode")? != *group_id {
                return Err(AppError::protocol());
            }
            let code: i64 = required(&value, "resultCode")?;
            if code != 0 {
                return Err(AppError::new(
                    "BACKEND_REJECTED",
                    "Backend rejected the group extension query.",
                    4,
                ));
            }
            let ext = value
                .get("extInfo")
                .filter(|v| v.is_object())
                .ok_or_else(AppError::protocol)?;
            let owner = match ext.get("groupOwnerId") {
                None | Some(Value::Null) => None,
                Some(owner) => {
                    if !owner.is_object() {
                        return Err(AppError::protocol());
                    }
                    Some(Owner {
                        user_id: owner
                            .get("memberUin")
                            .and_then(identifier)
                            .and_then(|v| v.parse().ok()),
                        uid: opt(owner, "memberUid")?,
                        qid: opt(owner, "memberQid")?,
                    })
                }
            };
            Envelope::typed(Extended {
                group_id: group_id.clone(),
                owner,
                robot_member_switch: opt(ext, "inviteRobotMemberSwitch")?,
                robot_member_examine: opt(ext, "inviteRobotMemberExamine")?,
                robot_switch: opt(ext, "inviteRobotSwitch")?,
                custom_portrait: opt(ext, "hasGroupCustomPortrait")?,
                essential_message_switch: opt(ext, "essentialMsgSwitch")?,
                group_square_switch: opt(ext, "groupSquareSwitch")?,
                lucky_word: opt(ext, "luckyWord")?,
                lucky_word_id: opt(ext, "luckyWordId")?,
                viewed_message_disappear_time: opt(ext, "viewedMsgDisappearTime")?,
            })
        }
        GroupCommand::Share { .. } => {
            let url: String = required(&value, "url")?;
            if url.is_empty() {
                return Err(AppError::protocol());
            }
            Envelope::typed(Share {
                group_id: group_id.clone(),
                url,
            })
        }
        GroupCommand::MentionQuota { .. } => Envelope::typed(Quota {
            group_id: group_id.clone(),
            can_at_all: required(&value, "can_at_all")?,
            remaining_for_group: required(&value, "remain_at_all_count_for_group")?,
            remaining_for_account: required(&value, "remain_at_all_count_for_uin")?,
        }),
        GroupCommand::Muted { .. } => {
            let rows = array(&value)?;
            budget(rows.len(), session.config.scan_limit)?;
            let members = rows
                .iter()
                .map(|row| {
                    Ok(MutedMember {
                        uid: required(row, "uid")?,
                        user_id: row
                            .get("uin")
                            .and_then(identifier)
                            .and_then(|v| v.parse().ok()),
                        nickname: opt(row, "nick")?,
                        card: opt(row, "cardName")?,
                        role_code: opt(row, "role")?,
                        shut_up_time: required(row, "shutUpTime")?,
                    })
                })
                .collect::<Result<_>>()?;
            Envelope::typed(Muted {
                group_id: group_id.clone(),
                members,
                complete: No,
            })
        }
        GroupCommand::Honors { kind, .. } => {
            if id(&value, "group_id")? != *group_id {
                return Err(AppError::protocol());
            }
            let talkative = honors(&value, "talkative_list")?;
            let performer = honors(&value, "performer_list")?;
            let legend = honors(&value, "legend_list")?;
            let emotion = honors(&value, "emotion_list")?;
            let strong_newbie = honors(&value, "strong_newbie_list")?;
            let current = value
                .get("current_talkative")
                .and_then(Value::as_object)
                .ok_or_else(AppError::protocol)?;
            let current_talkative = if current.is_empty() {
                None
            } else {
                Some(honor(
                    value
                        .get("current_talkative")
                        .ok_or_else(AppError::protocol)?,
                )?)
            };
            budget(
                talkative.len()
                    + performer.len()
                    + legend.len()
                    + emotion.len()
                    + strong_newbie.len()
                    + usize::from(current_talkative.is_some()),
                session.config.scan_limit,
            )?;
            Envelope::typed(Honors {
                group_id: group_id.clone(),
                kind: *kind,
                current_talkative,
                talkative,
                performer,
                legend,
                emotion,
                strong_newbie,
                complete: No,
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(envelope.scoped(scope)))
}

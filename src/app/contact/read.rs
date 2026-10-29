//! Typed projections from a bounded backend snapshot; unknown fields never become public DTOs.
use crate::{
    cli::contact::ContactCommand,
    core::{
        contracts::{
            account::NumberOrString, contact::*, group::BoolOrString, resource::Row, scalar::No,
        },
        domain::{Conversation, identifier},
        error::{AppError, Result},
        input::DecimalId,
        output::Envelope,
    },
    messaging::{read, session::Session},
    transport::client::{Backend, parse_raw_envelope},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Instant;
fn opt<T: DeserializeOwned>(value: &Value, key: &str) -> Result<Option<T>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|_| AppError::protocol()),
    }
}
fn required<T: DeserializeOwned>(value: &Value, key: &str) -> Result<T> {
    opt(value, key)?.ok_or_else(AppError::protocol)
}
fn id(value: &Value, key: &str) -> Result<DecimalId> {
    value
        .get(key)
        .and_then(identifier)
        .and_then(|id| id.parse().ok())
        .ok_or_else(AppError::protocol)
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or_else(AppError::protocol)
}
fn budget(size: usize, max: usize) -> Result<()> {
    if size > max {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Backend snapshot exceeds the configured scan_limit; no partial list was presented as complete.",
            4,
        ));
    }
    Ok(())
}
fn allowed<B: Backend>(session: &Session<'_, B>, id: &str) -> bool {
    let targets = &session.config.permissions.targets;
    targets.is_empty() || targets.contains(&format!("private:{id}"))
}
fn like_users(value: &Value) -> Result<Vec<LikeUser>> {
    array(value.get("userInfos").ok_or_else(AppError::protocol)?)?
        .iter()
        .map(|row| {
            Ok(LikeUser {
                uid: required(row, "uid")?,
                user_id: row
                    .get("uin")
                    .and_then(identifier)
                    .and_then(|id| id.parse().ok()),
            })
        })
        .collect()
}
pub(super) fn execute<B: Backend>(
    command: &ContactCommand,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let (action, payload, target) = match command {
        ContactCommand::Card {
            user_id,
            phone_number,
            ..
        } => (
            "ArkSharePeer",
            serde_json::to_value(crate::core::contracts::card::PeerPayload {
                user_id: Some(user_id.to_string()),
                group_id: None,
                phone_number: phone_number
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            }),
            Some(Conversation::Private(user_id.to_string())),
        ),
        ContactCommand::Status { user_id, .. } => (
            "nc_get_user_status",
            serde_json::to_value(UserPayload {
                user_id: user_id.to_string(),
            }),
            Some(Conversation::Private(user_id.to_string())),
        ),
        ContactCommand::Likes {
            user_id,
            start,
            limit,
            ..
        } => (
            "get_profile_like",
            serde_json::to_value(LikesPayload {
                user_id: user_id.as_ref().map(ToString::to_string),
                start: NumberOrString::Number(f64::from(*start)),
                count: NumberOrString::Number(f64::from(limit.get())),
            }),
            user_id
                .as_ref()
                .map(|id| Conversation::Private(id.to_string())),
        ),
        ContactCommand::Profile {
            user_id, no_cache, ..
        } => (
            "get_stranger_info",
            serde_json::to_value(ProfilePayload {
                user_id: user_id.to_string(),
                no_cache: BoolOrString::Bool(*no_cache),
            }),
            Some(Conversation::Private(user_id.to_string())),
        ),
        ContactCommand::Categories { .. } => (
            "get_friends_with_category",
            serde_json::to_value(EmptyPayload {}),
            None,
        ),
        ContactCommand::Unilateral { .. } => (
            "get_unidirectional_friend_list",
            serde_json::to_value(EmptyPayload {}),
            None,
        ),
        ContactCommand::Requests { limit, .. } => (
            "get_doubt_friends_add_request",
            serde_json::to_value(RequestsPayload {
                count: f64::from(limit.get()),
            }),
            None,
        ),
        _ => return Ok(None),
    };
    let payload = payload.map_err(|_| AppError::invalid())?;
    let (value, scope) = session.read(target.as_ref(), deadline, |backend| {
        parse_raw_envelope(backend.call_raw(
            &action.parse().map_err(|_| AppError::protocol())?,
            &payload,
            deadline,
        )?)
    })?;
    let filtered_by_policy = !session.config.permissions.targets.is_empty();
    let envelope = match command {
        ContactCommand::Card { user_id, .. } => {
            if required::<i64>(&value, "result")? != 0 {
                return Err(AppError::new(
                    "BACKEND_REJECTED",
                    "Backend rejected contact card generation.",
                    4,
                ));
            }
            let card_json = required::<String>(&value, "arkMsg")?;
            if card_json.is_empty() {
                return Err(AppError::new(
                    "CARD_UNAVAILABLE",
                    "Backend returned no card content.",
                    4,
                ));
            }
            Envelope::typed(crate::core::contracts::card::Card {
                conversation: Conversation::Private(user_id.to_string()),
                card_json,
            })
        }
        ContactCommand::Status { user_id, .. } => Envelope::typed(UserStatus {
            user_id: user_id.clone(),
            status: required(&value, "status")?,
            ext_status: required(&value, "ext_status")?,
        }),
        ContactCommand::Likes {
            user_id,
            expected_uid,
            start,
            limit,
            ..
        } => {
            let uid: String = required(&value, "uid")?;
            if expected_uid
                .as_ref()
                .is_some_and(|expected| **expected != uid)
            {
                return Err(AppError::new(
                    "SUBJECT_MISMATCH",
                    "Backend returned a different profile UID; no like counts were attributed to the requested user.",
                    4,
                ));
            }
            let favorite = value.get("favoriteInfo").ok_or_else(AppError::protocol)?;
            let vote = value.get("voteInfo").ok_or_else(AppError::protocol)?;
            let fusers = like_users(favorite)?;
            let vusers = like_users(vote)?;
            budget(
                fusers.len().saturating_add(vusers.len()),
                session.config.scan_limit,
            )?;
            Envelope::typed(Likes {
                user_id: match user_id {
                    Some(id) => id.clone(),
                    None => scope.account.parse().map_err(|_| AppError::protocol())?,
                },
                uid,
                time: required(&value, "time")?,
                favorite_info: FavoriteInfo {
                    total_count: required(favorite, "total_count")?,
                    last_time: required(favorite, "last_time")?,
                    today_count: required(favorite, "today_count")?,
                    users: fusers,
                },
                vote_info: VoteInfo {
                    total_count: required(vote, "total_count")?,
                    new_count: required(vote, "new_count")?,
                    new_nearby_count: required(vote, "new_nearby_count")?,
                    last_visit_time: required(vote, "last_visit_time")?,
                    users: vusers,
                },
                start: *start,
                requested_limit: usize::from(limit.get()),
                complete: No,
            })
        }
        ContactCommand::Profile { user_id, .. } => {
            let actual = id(&value, "user_id")?;
            if &actual != user_id {
                return Err(AppError::protocol());
            }
            Envelope::typed(Profile {
                user_id: actual,
                uid: opt(&value, "uid")?,
                nickname: opt(&value, "nickname")?,
                signature: opt(&value, "long_nick")?,
                remark: opt(&value, "remark")?,
                sex: opt(&value, "sex")?,
                qid: opt(&value, "qid")?,
                age: opt(&value, "age")?,
                level: opt(&value, "qqLevel")?,
                status: opt(&value, "status")?,
                registration_time: opt(&value, "reg_time")?,
                vip: opt(&value, "is_vip")?,
                yearly_vip: opt(&value, "is_years_vip")?,
                vip_level: opt(&value, "vip_level")?,
            })
        }
        ContactCommand::Categories { .. } => {
            let rows = array(&value)?;
            let mut used = rows.len();
            budget(used, session.config.scan_limit)?;
            let mut categories = Vec::new();
            for row in rows {
                let members = array(row.get("buddyList").ok_or_else(AppError::protocol)?)?;
                used = used
                    .checked_add(members.len())
                    .ok_or_else(AppError::protocol)?;
                budget(used, session.config.scan_limit)?;
                let mut friends = Vec::new();
                for value in members {
                    let Row::Contact(c) = read::contact(value)? else {
                        return Err(AppError::protocol());
                    };
                    if allowed(session, &c.user_id) {
                        friends.push(c);
                    }
                }
                categories.push(Category {
                    category_id: required(row, "categoryId")?,
                    name: required(row, "categoryName")?,
                    reported_member_count: required(row, "categoryMbCount")?,
                    friends,
                });
            }
            Envelope::typed(Categories {
                categories,
                filtered_by_policy,
                complete: No,
            })
        }
        ContactCommand::Unilateral { .. } => {
            let rows = array(&value)?;
            budget(rows.len(), session.config.scan_limit)?;
            let mut users = Vec::new();
            for row in rows {
                let user_id = id(row, "uin")?;
                if allowed(session, &user_id) {
                    users.push(UnilateralUser {
                        user_id,
                        uid: required(row, "uid")?,
                        nickname: required(row, "nick_name")?,
                        age: required(row, "age")?,
                        source: required(row, "source")?,
                    });
                }
            }
            Envelope::typed(Unilateral {
                users,
                filtered_by_policy,
                complete: No,
                upstream_batch_limit: 99,
            })
        }
        ContactCommand::Requests { limit, .. } => {
            let rows = array(&value)?;
            budget(
                rows.len(),
                session.config.scan_limit.min(usize::from(limit.get())),
            )?;
            let mut requests = Vec::new();
            for row in rows {
                let user_id = row
                    .get("uin")
                    .and_then(identifier)
                    .and_then(|v| v.parse::<DecimalId>().ok());
                if filtered_by_policy && !user_id.as_ref().is_some_and(|v| allowed(session, v)) {
                    continue;
                }
                requests.push(FriendRequest {
                    flag: required(row, "flag")?,
                    user_id,
                    nickname: opt(row, "nick")?,
                    reason: opt(row, "reason")?,
                    message: opt(row, "msg")?,
                    group_id: row.get("group_code").and_then(identifier),
                    time: row.get("time").and_then(identifier),
                });
            }
            Envelope::typed(Requests {
                requests,
                filtered_by_policy,
                complete: No,
                requested_limit: usize::from(limit.get()),
            })
        }
        _ => return Ok(None),
    };
    Ok(Some(envelope.scoped(scope)))
}

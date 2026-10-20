use std::time::Instant;

use crate::core::contracts::resource::{Contact, Group, Member, Recent, Row};
use serde_json::{Map, Value, json};

use crate::core::domain::{Conversation, decimal_id, identifier};
use crate::core::error::{AppError, Result};
use crate::transport::client::{Action, Backend};

fn object(value: &Value) -> Result<&Map<String, Value>> {
    value.as_object().ok_or_else(AppError::protocol)
}

fn positive(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(identifier)
        .and_then(|id| decimal_id(&id).ok())
        .ok_or_else(AppError::protocol)
}

fn optional_string(source: &Value, name: &str) -> Result<Option<String>> {
    match source.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        _ => Err(AppError::protocol()),
    }
}
fn optional_count(source: &Value, name: &str) -> Result<Option<u64>> {
    match source.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(AppError::protocol),
    }
}
pub fn contact(value: &Value) -> Result<Row> {
    object(value)?;
    let id = positive(value, "user_id")?;
    Ok(Row::Contact(Contact {
        conversation: format!("private:{id}"),
        user_id: id,
        nickname: optional_string(value, "nickname")?,
        remark: optional_string(value, "remark")?,
    }))
}
pub fn group(value: &Value) -> Result<Row> {
    object(value)?;
    let id = positive(value, "group_id")?;
    Ok(Row::Group(Group {
        conversation: format!("group:{id}"),
        group_id: id,
        name: optional_string(value, "group_name")?,
        remark: optional_string(value, "group_remark")?,
        member_count: optional_count(value, "member_count")?,
        max_member_count: optional_count(value, "max_member_count")?,
    }))
}
pub fn member(value: &Value) -> Result<Row> {
    object(value)?;
    Ok(Row::Member(Member {
        user_id: positive(value, "user_id")?,
        nickname: optional_string(value, "nickname")?,
        card: optional_string(value, "card")?,
        role: optional_string(value, "role")?,
    }))
}
/// Excludes lastestMsg: recent discovery is not a full message read.
pub fn recent(value: &Value) -> Result<Row> {
    object(value)?;
    let id = positive(value, "peerUin")?;
    let conversation = match value.get("chatType").and_then(Value::as_u64) {
        Some(1 | 100) => Conversation::Private(id),
        Some(2) => Conversation::Group(id),
        _ => {
            return Err(AppError::new(
                "CONVERSATION_TYPE_UNSUPPORTED",
                "Backend returned an unsupported recent conversation type.",
                4,
            ));
        }
    };
    let last_activity_seconds = value
        .get("msgTime")
        .map(|time| {
            identifier(time)
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|seconds| *seconds <= 253_402_300_799)
                .ok_or_else(AppError::protocol)
        })
        .transpose()?;
    Ok(Row::Recent(Recent {
        conversation: conversation.to_string(),
        name: optional_string(value, "peerName")?,
        remark: optional_string(value, "remark")?,
        last_activity_seconds,
    }))
}

pub fn contacts(backend: &impl Backend, deadline: Instant) -> Result<Vec<Row>> {
    normalize_list(
        backend.call(Action::FriendList, &json!({"no_cache":false}), deadline)?,
        contact,
    )
}

pub fn groups(backend: &impl Backend, deadline: Instant) -> Result<Vec<Row>> {
    normalize_list(
        backend.call(Action::GroupList, &json!({"no_cache":false}), deadline)?,
        group,
    )
}

pub fn members(backend: &impl Backend, group_id: &str, deadline: Instant) -> Result<Vec<Row>> {
    normalize_list(
        backend.call(
            Action::GroupMembers,
            &json!({"group_id":group_id,"no_cache":false}),
            deadline,
        )?,
        member,
    )
}

pub fn conversations(backend: &impl Backend, count: usize, deadline: Instant) -> Result<Vec<Row>> {
    normalize_list(
        backend.call(Action::RecentContacts, &json!({"count":count}), deadline)?,
        recent,
    )
}

fn normalize_list(value: Value, normalize: fn(&Value) -> Result<Row>) -> Result<Vec<Row>> {
    value
        .as_array()
        .ok_or_else(AppError::protocol)?
        .iter()
        .map(normalize)
        .collect()
}

pub fn filter(values: Vec<Row>, query: Option<&str>) -> Vec<Row> {
    match query {
        None => values,
        Some(query) => {
            let query = query.to_lowercase();
            values.into_iter().filter(|v| v.matches(&query)).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_models_do_not_leak_backend_fields_or_imply_unknown_facts() {
        let result =
            contact(&json!({"user_id":10001,"nickname":"Example","private_internal":"secret"}))
                .unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            json!({"userId":"10001","nickname":"Example","conversation":"private:10001"})
        );
        let group = serde_json::to_value(
            group(&json!({"group_id":"10002","group_name":"Example"})).unwrap(),
        )
        .unwrap();
        assert!(group.get("memberCount").is_none());
        assert!(contact(&json!({"user_id":1.5})).is_err());
        let recent = recent(&json!({"peerUin":"10002","chatType":2,"msgTime":"1700000000","lastestMsg":{"message":"private"}})).unwrap();
        let recent = serde_json::to_value(recent).unwrap();
        assert!(recent.get("lastestMsg").is_none());
        assert_eq!(recent["conversation"], "group:10002");
    }
}

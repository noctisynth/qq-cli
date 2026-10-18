use std::time::Instant;

use serde_json::{Map, Value, json};

use crate::client::{Action, Backend};
use crate::domain::{Conversation, decimal_id, identifier};
use crate::error::{AppError, Result};

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

fn optional_string(
    source: &Value,
    name: &str,
    output: &mut Map<String, Value>,
    key: &str,
) -> Result<()> {
    match source.get(name) {
        None | Some(Value::Null) => (),
        Some(Value::String(text)) => {
            output.insert(key.into(), json!(text));
        }
        _ => return Err(AppError::protocol()),
    }
    Ok(())
}

fn optional_count(
    source: &Value,
    name: &str,
    output: &mut Map<String, Value>,
    key: &str,
) -> Result<()> {
    match source.get(name) {
        None | Some(Value::Null) => (),
        Some(number) if number.as_u64().is_some() => {
            output.insert(key.into(), number.clone());
        }
        _ => return Err(AppError::protocol()),
    }
    Ok(())
}

pub fn contact(value: &Value) -> Result<Value> {
    object(value)?;
    let id = positive(value, "user_id")?;
    let mut out = Map::new();
    out.insert("userId".into(), json!(id));
    out.insert("conversation".into(), json!(format!("private:{id}")));
    optional_string(value, "nickname", &mut out, "nickname")?;
    optional_string(value, "remark", &mut out, "remark")?;
    Ok(Value::Object(out))
}

pub fn group(value: &Value) -> Result<Value> {
    object(value)?;
    let id = positive(value, "group_id")?;
    let mut out = Map::new();
    out.insert("groupId".into(), json!(id));
    out.insert("conversation".into(), json!(format!("group:{id}")));
    optional_string(value, "group_name", &mut out, "name")?;
    optional_string(value, "group_remark", &mut out, "remark")?;
    optional_count(value, "member_count", &mut out, "memberCount")?;
    optional_count(value, "max_member_count", &mut out, "maxMemberCount")?;
    Ok(Value::Object(out))
}

pub fn member(value: &Value) -> Result<Value> {
    object(value)?;
    let mut out = Map::new();
    out.insert("userId".into(), json!(positive(value, "user_id")?));
    optional_string(value, "nickname", &mut out, "nickname")?;
    optional_string(value, "card", &mut out, "card")?;
    optional_string(value, "role", &mut out, "role")?;
    Ok(Value::Object(out))
}

/// Deliberately excludes lastestMsg: a recent conversation is not a full message read.
pub fn recent(value: &Value) -> Result<Value> {
    object(value)?;
    let id = positive(value, "peerUin")?;
    let chat_type = value
        .get("chatType")
        .and_then(Value::as_u64)
        .ok_or_else(AppError::protocol)?;
    let conversation = match chat_type {
        1 | 100 => Conversation::Private(id),
        2 => Conversation::Group(id),
        _ => {
            return Err(AppError::new(
                "CONVERSATION_TYPE_UNSUPPORTED",
                "Backend returned an unsupported recent conversation type.",
                4,
            ));
        }
    };
    let mut out = Map::new();
    out.insert("conversation".into(), json!(conversation.to_string()));
    optional_string(value, "peerName", &mut out, "name")?;
    optional_string(value, "remark", &mut out, "remark")?;
    if let Some(time) = value.get("msgTime") {
        let seconds = identifier(time)
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|seconds| *seconds <= 253_402_300_799)
            .ok_or_else(AppError::protocol)?;
        out.insert("lastActivitySeconds".into(), json!(seconds));
    }
    Ok(Value::Object(out))
}

pub fn contacts(backend: &impl Backend, deadline: Instant) -> Result<Vec<Value>> {
    normalize_list(
        backend.call(Action::FriendList, &json!({"no_cache":false}), deadline)?,
        contact,
    )
}

pub fn groups(backend: &impl Backend, deadline: Instant) -> Result<Vec<Value>> {
    normalize_list(
        backend.call(Action::GroupList, &json!({"no_cache":false}), deadline)?,
        group,
    )
}

pub fn members(backend: &impl Backend, group_id: &str, deadline: Instant) -> Result<Vec<Value>> {
    normalize_list(
        backend.call(
            Action::GroupMembers,
            &json!({"group_id":group_id,"no_cache":false}),
            deadline,
        )?,
        member,
    )
}

pub fn conversations(
    backend: &impl Backend,
    count: usize,
    deadline: Instant,
) -> Result<Vec<Value>> {
    normalize_list(
        backend.call(Action::RecentContacts, &json!({"count":count}), deadline)?,
        recent,
    )
}

fn normalize_list(value: Value, normalize: fn(&Value) -> Result<Value>) -> Result<Vec<Value>> {
    value
        .as_array()
        .ok_or_else(AppError::protocol)?
        .iter()
        .map(normalize)
        .collect()
}

pub fn filter(values: Vec<Value>, query: Option<&str>) -> Vec<Value> {
    match query {
        None => values,
        Some(query) => {
            let query = query.to_lowercase();
            values
                .into_iter()
                .filter(|value| {
                    value.as_object().is_some_and(|fields| {
                        fields
                            .iter()
                            .filter(|(key, _)| key.as_str() != "conversation")
                            .any(|(_, value)| {
                                value
                                    .as_str()
                                    .is_some_and(|text| text.to_lowercase().contains(&query))
                            })
                    })
                })
                .collect()
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
            result,
            json!({"userId":"10001","nickname":"Example","conversation":"private:10001"})
        );
        let group = group(&json!({"group_id":"10002","group_name":"Example"})).unwrap();
        assert!(group.get("memberCount").is_none());
        assert!(contact(&json!({"user_id":1.5})).is_err());
        let recent = recent(&json!({"peerUin":"10002","chatType":2,"msgTime":"1700000000","lastestMsg":{"message":"private"}})).unwrap();
        assert!(recent.get("lastestMsg").is_none());
        assert_eq!(recent["conversation"], "group:10002");
    }
}

//! Message action payloads remain distinct from the existing send/reply contract.
use super::{
    account::NumberOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::domain::Conversation;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct MessagePayload {
    pub message_id: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}
#[derive(Serialize, JsonSchema)]
pub struct MarkReadPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub user_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message_id: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct ForwardPayload {
    pub message_id: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub user_id: Option<String>,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "message.recall")]
    Recall,
    #[serde(rename = "message.mark-read")]
    MarkRead,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub all_conversations: bool,
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Conversation")]
    pub conversation: Option<Conversation>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub all_conversations: bool,
    pub operation: Operation,
    pub dry_run: Yes,
    pub identity_verified: No,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Conversation")]
    pub conversation: Option<Conversation>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}

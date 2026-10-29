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
    #[serde(rename = "message.react")]
    React,
    #[serde(rename = "message.transcribe")]
    Transcribe,
    #[serde(rename = "message.forward")]
    Forward,
    #[serde(rename = "message.recall")]
    Recall,
    #[serde(rename = "message.mark-read")]
    MarkRead,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub may_send_self_copies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "super::write::ReceiptIdentity")]
    pub message: Option<super::write::ReceiptIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub merged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize", range(min = 1, max = 32))]
    pub source_messages: Option<usize>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub may_send_self_copies: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub merged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "usize", range(min = 1, max = 32))]
    pub source_messages: Option<usize>,
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

// This is the referenced-node subset emitted by the domain task, not the full native mix schema.
#[derive(Serialize, JsonSchema)]
pub enum ReferenceKind {
    #[serde(rename = "node")]
    Node,
}
#[derive(Serialize, JsonSchema)]
pub struct ReferenceData {
    pub id: String,
    pub nickname: String,
    pub content: [(); 0],
}
#[derive(Serialize, JsonSchema)]
pub struct ReferencedNode {
    #[serde(rename = "type")]
    pub kind: ReferenceKind,
    pub data: ReferenceData,
}
#[derive(Serialize, JsonSchema)]
pub struct ForwardBundlePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub user_id: Option<String>,
    pub message: Vec<ReferencedNode>,
}

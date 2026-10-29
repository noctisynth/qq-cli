//! Native online IDs are observations, never m1/f1 or local filesystem authority.
use super::{
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::domain::Conversation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, JsonSchema)]
pub struct UserPayload {
    pub user_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct SendPayload {
    pub user_id: String,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file_name: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct FolderPayload {
    pub user_id: String,
    pub folder_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder_name: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct CancelPayload {
    pub user_id: String,
    pub msg_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct ElementPayload {
    pub user_id: String,
    pub msg_id: String,
    pub element_id: String,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "online.send")]
    Send,
    #[serde(rename = "online.send-folder")]
    SendFolder,
    #[serde(rename = "online.receive")]
    Receive,
    #[serde(rename = "online.refuse")]
    Refuse,
    #[serde(rename = "online.cancel")]
    Cancel,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub dry_run: Yes,
    pub identity_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub conversation: Conversation,
    pub outcome: Outcome,
    pub deduplicated: bool,
    pub result_available: bool,
    pub transfer_verified: No,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_msg_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum Kind {
    File,
    Directory,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub native_msg_id: String,
    pub element_id: String,
    pub native_time: String,
    pub kind: Kind,
    pub name: String,
    pub native_size: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listing {
    pub conversation: Conversation,
    pub items: Vec<Item>,
    pub complete: No,
    pub truncated: bool,
}
#[derive(Deserialize)]
pub(crate) struct Sent {
    #[serde(rename = "msgId")]
    pub msg_id: String,
    #[serde(rename = "elementId")]
    pub element_id: String,
}

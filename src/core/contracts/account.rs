//! Account task receipts and the actual serialized backend payloads.
use super::{
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    Text(String),
}
#[derive(Serialize, JsonSchema)]
pub struct ProfilePayload {
    pub nickname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub personal_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub sex: Option<NumberOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct SignaturePayload {
    #[serde(rename = "longNick")]
    pub signature: String,
}
#[derive(Serialize, JsonSchema)]
pub struct AvatarPayload {
    pub file: String,
}
#[derive(Serialize, JsonSchema)]
pub struct PresencePayload {
    pub status: NumberOrString,
    pub ext_status: NumberOrString,
    pub battery_status: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct StatusTextPayload {
    pub face_id: NumberOrString,
    pub face_type: NumberOrString,
    pub wording: String,
}
#[derive(Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Field {
    Nickname,
    Signature,
    Sex,
    Avatar,
    Status,
    ExtStatus,
    BatteryStatus,
    StatusText,
    FaceId,
    FaceType,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "account.set")]
    Set,
    #[serde(rename = "account.avatar")]
    Avatar,
    #[serde(rename = "account.presence")]
    Presence,
    #[serde(rename = "account.status-text")]
    StatusText,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    pub requested_fields: Vec<Field>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub dry_run: Yes,
    pub identity_verified: No,
    pub requested_fields: Vec<Field>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_bytes: Option<u64>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}

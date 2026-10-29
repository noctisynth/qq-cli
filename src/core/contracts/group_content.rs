//! Group message administration and the upstream first sign-in page.
use super::{account::NumberOrString, scalar::No};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct GroupPayload {
    pub group_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct SignedPayload {
    pub group_id: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct DeleteEssencePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub message_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub msg_seq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub msg_random: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct TodoPayload {
    pub group_id: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message_seq: Option<String>,
}
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoState {
    Open,
    Complete,
    Cancel,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Signer {
    pub user_id: crate::core::input::DecimalId,
    pub nickname: String,
    pub native_time: f64,
    pub rank: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Signed {
    pub group_id: crate::core::input::DecimalId,
    pub complete: No,
    pub users: Vec<Signer>,
}

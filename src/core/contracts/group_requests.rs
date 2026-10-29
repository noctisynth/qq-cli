//! Group request batches and exact-action decision receipts.
use super::{
    account::NumberOrString,
    group::BoolOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::input::DecimalId;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}
#[derive(Serialize, JsonSchema)]
pub struct BatchPayload {
    pub count: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct RespondPayload {
    pub flag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub approve: Option<BoolOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub reason: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub count: Option<f64>,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Source {
    #[serde(rename = "get_group_system_msg")]
    System,
    #[serde(rename = "get_group_ignored_notifies")]
    LegacyNotices,
    #[serde(rename = "get_group_ignore_add_request")]
    Doubt,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Invite,
    Join,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub kind: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub observed_request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub request_flag: Option<String>,
    pub request_id_exact: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub group_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub actor_id: Option<DecimalId>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Batch {
    pub source: Source,
    pub requests: Vec<Request>,
    pub requested_limit: usize,
    pub source_batch_size: usize,
    pub filtered_by_policy: bool,
    pub truncated: bool,
    pub complete: No,
}
#[derive(Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "group.respond")]
    Respond,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub dry_run: Yes,
    pub identity_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum WriteReply {
    Receipt(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}

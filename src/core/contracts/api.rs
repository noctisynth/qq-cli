//! Explicit API discovery contracts; a catalog entry is not runtime acceptance.
use schemars::JsonSchema;
use serde::Serialize;
pub mod json;
pub mod stream;

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub dry_run: super::scalar::Yes,
    pub action: String,
    pub effect: Effect,
    pub catalog_known: bool,
    pub parameter_bytes: usize,
    pub identity_verified: super::scalar::No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Call {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "super::write::ReceiptIdentity")]
    pub receipt: Option<super::write::ReceiptIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "super::scalar::Yes")]
    pub compatibility_only: Option<super::scalar::Yes>,
    pub action: String,
    pub effect: Effect,
    pub catalog_known: bool,
    pub deduplicated: bool,
    pub result_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<super::write::Outcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<json::Document>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recorded {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "super::write::ReceiptIdentity")]
    pub receipt: Option<super::write::ReceiptIdentity>,
    pub request_id: String,
    pub operation: ApiOperation,
    pub action: String,
    pub outcome: super::write::Outcome,
    pub result_available: super::scalar::No,
}
#[derive(Serialize, JsonSchema)]
pub enum ApiOperation {
    #[serde(rename = "api.call")]
    Call,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failed {
    pub schema_version: super::scalar::WireVersion,
    pub ok: super::scalar::No,
    pub data: Call,
    pub error: crate::core::error::AppError,
    pub meta: super::reply::Meta,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum CallReply {
    Success(super::reply::Success<Call, super::reply::Scoped>),
    Failed(Failed),
    Error(super::reply::Failure),
}

#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Response {
    Call(CallReply),
    Draft(super::reply::Reply<Draft>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Read,
    Write,
    Unknown,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub action: String,
    pub effect: Effect,
    pub effect_verified: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub items: Vec<Summary>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub source_version: &'static str,
    pub source_commit: &'static str,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Description {
    pub response_mode: ResponseMode,
    pub compatibility_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "super::discovery::SchemaDocument")]
    pub parameter_schema: Option<super::discovery::SchemaDocument>,
    pub action: String,
    pub base_action: String,
    pub known: bool,
    pub effect: Effect,
    pub effect_verified: bool,
    pub request_id_required: bool,
    pub parameters_known: bool,
    pub parameter_shape: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResponseMode {
    Json,
    Stream,
    Unknown,
}

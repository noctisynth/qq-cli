//! Explicit API discovery contracts; a catalog entry is not runtime acceptance.
use schemars::JsonSchema;
use serde::Serialize;

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

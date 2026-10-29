//! QQ-space protocol inputs and bounded domain receipts.
use super::{
    account::NumberOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, JsonSchema, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Audience {
    Everyone,
    Friends,
    Selected,
    #[serde(rename = "self")]
    #[value(name = "self")]
    SelfOnly,
    Exclude,
}
impl Audience {
    pub fn code(self) -> u32 {
        match self {
            Self::Everyone => 1,
            Self::Friends => 4,
            Self::Selected => 16,
            Self::SelfOnly => 64,
            Self::Exclude => 128,
        }
    }
}
#[derive(Serialize, JsonSchema)]
pub struct PostPayload {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub ugc_right: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<NumberOrString>")]
    pub target_uins: Option<Vec<NumberOrString>>,
}
#[derive(Serialize, JsonSchema)]
pub struct DeletePayload {
    pub tid: String,
}
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "qzone.post")]
    Post,
    #[serde(rename = "qzone.delete")]
    Delete,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tid: Option<String>,
    pub tid_available: bool,
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
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}
#[derive(Deserialize)]
pub(crate) struct Published {
    pub tid: String,
}

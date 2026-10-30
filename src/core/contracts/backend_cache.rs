//! Upstream cleanup acknowledgements cannot prove every unlink succeeded.
use super::{
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "napcat.clean-cache")]
    CleanCache,
}
#[derive(Serialize, JsonSchema)]
pub enum Cache {
    #[serde(rename = "backend_and_qq_media")]
    BackendAndQqMedia,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub cache: Cache,
    pub dry_run: Yes,
    pub identity_verified: No,
    pub cleanup_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub cache: Cache,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    pub cleanup_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}

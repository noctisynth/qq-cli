//! Reviewed group album payloads and narrow domain projections.
use super::{
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::input::DecimalId;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct ListPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub attach_info: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct MediaPayload {
    pub group_id: String,
    pub album_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub attach_info: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct RemovePayload {
    pub group_id: String,
    pub album_id: String,
    pub lloc: String,
}
#[derive(Serialize, JsonSchema)]
pub struct LikePayload {
    pub group_id: String,
    pub album_id: String,
    pub batch_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub lloc: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct CommentPayload {
    pub group_id: String,
    pub album_id: String,
    pub lloc: String,
    pub content: String,
}
#[derive(Serialize, JsonSchema)]
pub struct UploadPayload {
    pub group_id: String,
    pub album_id: String,
    pub album_name: String,
    pub file: String,
}
#[derive(Serialize, JsonSchema)]
pub struct UploadManyPayload {
    pub group_id: String,
    pub album_id: String,
    pub album_name: String,
    pub files: Vec<String>,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "album.remove")]
    Remove,
    #[serde(rename = "album.like")]
    Like,
    #[serde(rename = "album.comment")]
    Comment,
    #[serde(rename = "album.upload")]
    Upload,
    #[serde(rename = "album.upload-video")]
    Video,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub group_id: DecimalId,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub group_id: DecimalId,
    pub dry_run: Yes,
    pub identity_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Album {
    pub album_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Albums {
    pub group_id: DecimalId,
    pub items: Vec<Album>,
    pub complete: No,
    pub truncated: bool,
    pub native_has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_attach_info: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Media {
    pub media_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Medias {
    pub group_id: DecimalId,
    pub album_id: String,
    pub items: Vec<Media>,
    pub complete: No,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_attach_info: Option<String>,
}

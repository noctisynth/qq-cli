//! File protocol inputs preserve native ID aliases; no backend path is a local file capability.
use super::{
    account::NumberOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::domain::Conversation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, JsonSchema)]
pub struct GroupPayload {
    pub group_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct GroupFilePayload {
    pub group_id: String,
    pub file_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct PrivateUrlPayload {
    pub file_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct RootPayload {
    pub group_id: String,
    pub file_count: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct FolderPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct FolderListPayload {
    #[serde(flatten)]
    pub folder: FolderPayload,
    pub file_count: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct MkdirPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct MovePayload {
    pub group_id: String,
    pub file_id: String,
    pub current_parent_directory: String,
    pub target_parent_directory: String,
}
#[derive(Serialize, JsonSchema)]
pub struct RenamePayload {
    pub group_id: String,
    pub file_id: String,
    pub current_parent_directory: String,
    pub new_name: String,
}
#[derive(Serialize, JsonSchema)]
pub struct PrivateUploadPayload {
    pub user_id: String,
    pub file: String,
    pub name: String,
    pub upload_file: bool,
}
#[derive(Serialize, JsonSchema)]
pub struct GroupUploadPayload {
    pub group_id: String,
    pub file: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub folder_id: Option<String>,
    pub upload_file: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Headers {
    One(String),
    Many(Vec<String>),
}
#[derive(Serialize, JsonSchema)]
pub struct CachePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Headers")]
    pub headers: Option<Headers>,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "file.mkdir")]
    Mkdir,
    #[serde(rename = "file.rmdir")]
    Rmdir,
    #[serde(rename = "file.remove")]
    Remove,
    #[serde(rename = "file.move")]
    Move,
    #[serde(rename = "file.rename")]
    Rename,
    #[serde(rename = "file.transfer")]
    Transfer,
    #[serde(rename = "file.upload")]
    Upload,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub conversation: Conversation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_file_id: Option<String>,
    pub file_id_available: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub operation: Operation,
    pub conversation: Conversation,
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
pub struct Info {
    pub conversation: Conversation,
    pub native_file_count: f64,
    pub native_limit_count: f64,
    pub native_used_space: f64,
    pub native_total_space: f64,
    pub quota_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct File {
    pub file_id: String,
    pub name: String,
    pub native_size: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploader_id: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Folder {
    pub folder_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_file_count: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listing {
    pub conversation: Conversation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub files: Vec<File>,
    pub folders: Vec<Folder>,
    pub complete: No,
    pub truncated: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Url {
    pub conversation: Conversation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub url_available: bool,
}
#[derive(Deserialize)]
pub(crate) struct Uploaded {
    pub file_id: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub enum StageOperation {
    #[serde(rename = "file.stage")]
    Stage,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageReceipt {
    pub operation: StageOperation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_path: Option<String>,
    pub path_available: bool,
    pub size_bytes: u64,
    pub sha256: String,
    pub requested_retention_seconds: u32,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StageDraft {
    pub operation: StageOperation,
    pub dry_run: Yes,
    pub identity_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum StageReply {
    Result(super::reply::ScopedReply<StageReceipt>),
    Draft(super::reply::Reply<StageDraft>),
}

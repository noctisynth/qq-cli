//! File-set contracts retain native status/size units and exclude backend paths.
use super::{
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::domain::Conversation;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Serialize, JsonSchema, clap::ValueEnum)]
pub enum Validity {
    #[serde(rename = "14")]
    #[value(name = "14")]
    Days14,
    #[serde(rename = "90")]
    #[value(name = "90")]
    Days90,
    #[serde(rename = "180")]
    #[value(name = "180")]
    Days180,
}
impl Validity {
    pub fn seconds(self) -> u32 {
        match self {
            Self::Days14 => 1209600,
            Self::Days90 => 7776000,
            Self::Days180 => 15552000,
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Paths {
    One(String),
    Many(Vec<String>),
}
#[derive(Serialize, JsonSchema)]
pub struct CreatePayload {
    pub files: Paths,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub thumb_path: Option<String>,
    #[serde(rename = "validitySeconds", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub validity_seconds: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
pub struct SetPayload {
    pub fileset_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct ResolvePayload {
    pub share_code: String,
}
#[derive(Serialize, JsonSchema)]
pub struct UrlPayload {
    pub fileset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub file_index: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
pub struct SendPayload {
    pub fileset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "flash.create")]
    Create,
    #[serde(rename = "flash.send")]
    Send,
    #[serde(rename = "flash.download")]
    Download,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    pub result_available: bool,
    pub transfer_verified: No,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fileset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_expiry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Conversation>,
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
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Counts {
    pub succeeded: f64,
    pub failed: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Info {
    pub fileset_id: String,
    pub name: String,
    pub native_file_count: f64,
    pub native_total_size: f64,
    pub native_status: f64,
    pub native_upload_status: f64,
    pub native_download_status: f64,
    pub upload: Counts,
    pub download: Counts,
    pub transfer_verified: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct File {
    pub client_id: String,
    pub name: String,
    pub directory: bool,
    pub native_size: String,
    pub batch_index: usize,
    pub native_client_index: f64,
    pub native_upload_status: f64,
    pub native_download_status: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listing {
    pub fileset_id: String,
    pub items: Vec<File>,
    pub complete: No,
    pub truncated: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Resolved {
    pub fileset_id: String,
    pub verification_available: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    pub fileset_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub url_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_expiry: Option<String>,
}
#[derive(Deserialize)]
pub(crate) struct Created {
    #[serde(rename = "createFlashTransferResult")]
    pub result: CreatedSet,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatedSet {
    pub file_set_id: String,
    pub share_link: Option<String>,
    pub expire_time: Option<String>,
}

//! Fixed native stream payload shapes, independent of CLI streaming output.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct DownloadPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub chunk_size: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
pub struct RecordPayload {
    #[serde(flatten)]
    pub download: DownloadPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub out_format: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct UploadPayload {
    pub stream_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub chunk_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub chunk_index: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub total_chunks: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub file_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub expected_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub is_complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub reset: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub verify_only: Option<bool>,
    pub file_retention: f64,
}
#[derive(Serialize, JsonSchema)]
pub struct TestPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub error: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}

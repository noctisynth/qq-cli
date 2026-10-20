//! Offline discovery DTOs. Only SchemaDocument is an open protocol document.
use super::scalar::No;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub struct SchemaDocument(
    #[schemars(with = "std::collections::BTreeMap<String,serde_json::Value>")] schemars::Schema,
);
impl SchemaDocument {
    pub fn new(value: serde_json::Value) -> crate::core::error::Result<Self> {
        if !value.is_object() {
            return Err(crate::core::error::AppError::protocol());
        }
        schemars::Schema::try_from(value)
            .map(Self)
            .map_err(|_| crate::core::error::AppError::protocol())
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandSummary {
    pub name: String,
    pub summary: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Defaults {
    pub automatic_instance_selection: Selection,
    pub automatic_write_retry: No,
}
#[derive(Serialize, JsonSchema)]
pub enum Selection {
    #[serde(rename = "only one candidate")]
    OnlyOne,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Ndjson,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamProtocol {
    pub ready: &'static str,
    pub pre_ready_failure: &'static str,
    pub normal_termination: &'static str,
    pub output_failure: &'static str,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Description {
    pub name: String,
    pub input_schema: SchemaDocument,
    pub response_schema: SchemaDocument,
    pub side_effects: &'static str,
    pub request_id: &'static str,
    pub defaults: Defaults,
    pub output_format: OutputFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "SchemaDocument")]
    pub pre_stream_error_schema: Option<SchemaDocument>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "StreamProtocol")]
    pub stream_protocol: Option<StreamProtocol>,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillSummary {
    pub task: &'static str,
    pub summary: &'static str,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillGuide {
    pub task: &'static str,
    pub commands: &'static [&'static str],
    pub instructions: String,
}
#[derive(Serialize, JsonSchema)]
pub enum SkillName {
    #[serde(rename = "qq-cli")]
    QqCli,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillExport {
    pub output: std::path::PathBuf,
    pub skill: SkillName,
    pub version: &'static str,
    pub files: usize,
}

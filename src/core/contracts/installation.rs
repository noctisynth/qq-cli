//! Install artifacts and recorded instance observations, never claims of a live identity.
use super::scalar::{LocalId, No, Yes};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Installed {
    pub installation_id: LocalId,
    pub version: String,
    pub cached: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Uninstalled {
    pub installation_id: LocalId,
    pub removed: Yes,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Installation {
    pub installation_id: LocalId,
    pub version: String,
    pub references: usize,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordedInstance {
    pub instance_id: LocalId,
    pub recorded_state: String,
    pub installation_id: LocalId,
    pub identity_verified: No,
}

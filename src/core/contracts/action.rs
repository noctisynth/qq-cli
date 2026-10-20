//! Curated machine-action hints, never arbitrary commands or arguments.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "command", content = "args", deny_unknown_fields)]
pub enum NextAction {
    #[serde(rename = "status")]
    Status { instance: String },
    #[serde(rename = "napcat.instances")]
    Instances {},
    #[serde(rename = "doctor")]
    Doctor {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        instance: Option<String>,
    },
    #[serde(rename = "media.prune")]
    Prune {},
    #[serde(rename = "request.get")]
    Request { request_id: String },
    #[serde(rename = "napcat.install")]
    Install { version: String },
    #[serde(rename = "napcat.start")]
    Start { instance: String },
    #[serde(rename = "napcat.stop")]
    Stop { instance: String },
    #[serde(rename = "napcat.remove")]
    Remove { instance_id: String },
    #[serde(rename = "napcat.update")]
    Update { instance: String, version: String },
    #[serde(rename = "napcat.rollback")]
    Rollback { instance: String },
}

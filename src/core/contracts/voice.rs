//! Explicit record retrieval and a once-only transcription result.
use super::{account::NumberOrString, scalar::LocalId, write::Outcome};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    #[default]
    Original,
    Mp3,
    Amr,
    Wma,
    M4a,
    Spx,
    Ogg,
    Wav,
    Flac,
}
impl Format {
    pub fn wire(self) -> &'static str {
        match self {
            Self::Original => "",
            Self::Mp3 => "mp3",
            Self::Amr => "amr",
            Self::Wma => "wma",
            Self::M4a => "m4a",
            Self::Spx => "spx",
            Self::Ogg => "ogg",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}
#[derive(Serialize, JsonSchema)]
pub struct RecordPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub file_id: Option<String>,
    pub out_format: String,
}
#[derive(Serialize, JsonSchema)]
pub struct TextPayload {
    pub message_id: NumberOrString,
}
#[derive(Deserialize)]
pub struct TextResult {
    pub text: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transcript {
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
    pub conversation: crate::core::domain::Conversation,
    pub text_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String", length(min = 1))]
    pub text: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Result(super::reply::ScopedReply<Transcript>),
    Draft(super::reply::Reply<super::message_ops::Draft>),
}

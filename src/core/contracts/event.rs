//! Actual tagged NDJSON record protocol; upstream frames remain a separate boundary.
use super::{
    message::Message,
    scalar::{No, WireVersion, Yes},
};
use crate::core::domain::Scope;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    BestEffort,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GapCoverage {
    Unknown,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Live,
    History,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Incoming,
    Outgoing,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    Ready {
        schema_version: WireVersion,
        scope: Scope,
        self_messages: Option<bool>,
        coverage: Coverage,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^w1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        checkpoint: Option<String>,
    },
    Message {
        #[schemars(regex(pattern = "^[a-f0-9]{64}$"))]
        event_id: String,
        source: Source,
        direction: Direction,
        message: Box<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^w1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        checkpoint: Option<String>,
    },
    Notice {
        #[schemars(regex(pattern = "^[a-f0-9]{64}$"))]
        event_id: String,
        #[serde(flatten)]
        content: NoticeContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^w1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        checkpoint: Option<String>,
    },
    Gap {
        reason: String,
        recovered: No,
        coverage: GapCoverage,
    },
    Summary {
        reason: String,
        emitted: usize,
        duplicates: usize,
        filtered: usize,
        heartbeats: usize,
        frames: usize,
        gaps: usize,
        bytes: usize,
        coverage: Coverage,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        error_code: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            with = "String",
            regex(pattern = "^w1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$")
        )]
        checkpoint: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Recovery")]
        recovery: Option<Recovery>,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum NoticeContent {
    Known {
        #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
        conversation: String,
        supported: Yes,
        notice: Notice,
    },
    Unsupported {
        #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
        conversation: Option<String>,
        supported: No,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Notice {
    GroupRecall {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: Option<String>,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
        message_id: String,
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        operator_id: Option<String>,
    },
    FriendRecall {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: String,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
        message_id: String,
    },
    FriendAdd {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: String,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
    },
    GroupIncrease {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: Option<String>,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        operator_id: Option<String>,
        sub_type: Increase,
    },
    GroupDecrease {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: Option<String>,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        operator_id: Option<String>,
        sub_type: Decrease,
    },
    GroupAdmin {
        #[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))]
        user_id: Option<String>,
        #[schemars(range(max = 253402300799_u64))]
        time_seconds: u64,
        sub_type: Admin,
    },
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Increase {
    Approve,
    Invite,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Decrease {
    Leave,
    Kick,
    KickMe,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Admin {
    Set,
    Unset,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Recovery {
    pub attempts: u64,
    pub scanned: u64,
    pub emitted: u64,
    #[schemars(range(min = 0, max = 253402300799_i64))]
    pub since_seconds: i64,
    pub stop_reason: super::history::StopReason,
    pub history_complete: bool,
    pub coverage: super::history::Coverage,
}
impl Event {
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Message { event_id, .. } | Self::Notice { event_id, .. } => Some(event_id),
            _ => None,
        }
    }
    pub fn set_checkpoint(&mut self, value: String) {
        match self {
            Self::Ready { checkpoint, .. }
            | Self::Message { checkpoint, .. }
            | Self::Notice { checkpoint, .. }
            | Self::Summary { checkpoint, .. } => *checkpoint = Some(value),
            Self::Gap { .. } => {}
        }
    }
    pub fn set_error(&mut self, value: &str) {
        if let Self::Summary { error_code, .. } = self {
            *error_code = Some(value.into())
        }
    }
}

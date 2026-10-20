//! Actual instance lifecycle and login-observation wire contracts.
use super::scalar::{LocalId, No};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    WaitingQrcode,
    GeneratingQrcode,
    QrcodeScanned,
    Initializing,
    Ready,
    Offline,
    Reconnecting,
    Unknown,
}
#[derive(Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation {
    pub phase: Phase,
    pub is_login: bool,
    pub is_offline: bool,
    pub qr_login_accepted: bool,
    pub core_ready: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservedState {
    Ready,
    Offline,
    AccountChanged,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PendingState {
    NotStarted,
    Preparing,
    Prepared,
    Starting,
    LoginRequired,
    Stopped,
    Stopping,
    Offline,
    Failed,
    Unknown,
}
impl PendingState {
    pub fn parse(value: &str) -> crate::core::error::Result<Self> {
        Ok(match value {
            "not_started" => Self::NotStarted,
            "preparing" => Self::Preparing,
            "prepared" => Self::Prepared,
            "starting" => Self::Starting,
            "login_required" => Self::LoginRequired,
            "stopped" => Self::Stopped,
            "stopping" => Self::Stopping,
            "offline" => Self::Offline,
            "failed" => Self::Failed,
            "unknown" => Self::Unknown,
            _ => return Err(crate::core::error::AppError::storage()),
        })
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observed {
    pub state: ObservedState,
    pub online: bool,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pending {
    pub state: PendingState,
    pub account: (),
    pub identity_verified: No,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Observation")]
    pub login_observation: Option<Observation>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Status {
    Observed(Observed),
    Pending(Pending),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    pub instance_id: LocalId,
    #[serde(flatten)]
    pub status: Observed,
    pub reused: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoppedState {
    Stopped,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RemovedState {
    Removed,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stopped {
    pub instance_id: LocalId,
    pub state: StoppedState,
    pub reused: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Removed {
    pub instance_id: LocalId,
    pub state: RemovedState,
    pub reused: bool,
}

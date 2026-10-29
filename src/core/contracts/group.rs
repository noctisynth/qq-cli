//! Serialized group protocol payloads and strict user task results.
use super::{
    account::NumberOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::input::DecimalId;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum BoolOrString {
    Bool(bool),
    Text(String),
}
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitePolicy {
    Disabled,
    RequireApproval,
    NoApproval,
    NoApprovalUnder100,
}
#[derive(Serialize, JsonSchema)]
pub struct RenamePayload {
    pub group_id: String,
    pub group_name: String,
}
#[derive(Serialize, JsonSchema)]
pub struct CardPayload {
    pub group_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub card: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct AdminPayload {
    pub group_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub enable: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct BanPayload {
    pub group_id: String,
    pub user_id: String,
    pub duration: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct BanAllPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub enable: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct KickPayload {
    pub group_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub reject_add_request: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct KickManyPayload {
    pub group_id: String,
    pub user_id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub reject_add_request: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct LeavePayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub is_dismiss: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct TitlePayload {
    pub group_id: String,
    pub user_id: String,
    pub special_title: String,
}
#[derive(Serialize, JsonSchema)]
pub struct AvatarPayload {
    pub group_id: String,
    pub file: String,
}
#[derive(Serialize, JsonSchema)]
pub struct RemarkPayload {
    pub group_id: String,
    pub remark: String,
}
#[derive(Serialize, JsonSchema)]
pub struct RobotPolicyPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub robot_member_switch: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub robot_member_examine: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
pub struct JoinPolicyPayload {
    pub group_id: String,
    pub add_type: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_answer: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct SearchPolicyPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub no_code_finger_open: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub no_finger_open: Option<f64>,
}
#[derive(Serialize, JsonSchema)]
pub struct InvitePolicyPayload {
    pub group_id: String,
    pub policy: InvitePolicy,
}
#[derive(Serialize, JsonSchema)]
pub struct MemberPermissionsPayload {
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub allow_member_upload_album: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub allow_member_temporary_session: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub allow_member_create_group: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
pub struct HistoryVisibilityPayload {
    pub group_id: String,
    pub visible: bool,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "group.poke")]
    Poke,
    #[serde(rename = "group.rename")]
    Rename,
    #[serde(rename = "group.card")]
    Card,
    #[serde(rename = "group.admin")]
    Admin,
    #[serde(rename = "group.ban")]
    Ban,
    #[serde(rename = "group.ban-all")]
    BanAll,
    #[serde(rename = "group.kick")]
    Kick,
    #[serde(rename = "group.kick-many")]
    KickMany,
    #[serde(rename = "group.leave")]
    Leave,
    #[serde(rename = "group.dismiss")]
    Dismiss,
    #[serde(rename = "group.title")]
    Title,
    #[serde(rename = "group.avatar")]
    Avatar,
    #[serde(rename = "group.remark")]
    Remark,
    #[serde(rename = "group.robot-policy")]
    RobotPolicy,
    #[serde(rename = "group.join-policy")]
    JoinPolicy,
    #[serde(rename = "group.search-policy")]
    SearchPolicy,
    #[serde(rename = "group.invite-policy")]
    InvitePolicy,
    #[serde(rename = "group.member-permissions")]
    MemberPermissions,
    #[serde(rename = "group.history-visibility")]
    HistoryVisibility,
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

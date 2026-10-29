//! Reviewed group read payloads and projections, separate from write receipts.
use super::{account::NumberOrString, group::BoolOrString, resource::Member, scalar::No};
use crate::core::input::DecimalId;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HonorKind {
    All,
    Talkative,
    Performer,
    Legend,
    Emotion,
    StrongNewbie,
}
#[derive(Serialize, JsonSchema)]
pub struct GroupPayload {
    pub group_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct MemberPayload {
    pub group_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub no_cache: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct SharePayload {
    pub group_id: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub need_short_url: Option<BoolOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub src_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub additional_param: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct HonorsPayload {
    pub group_id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HonorKind")]
    pub kind: Option<HonorKind>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberDetails {
    pub group_id: DecimalId,
    #[serde(flatten)]
    pub member: Member,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub join_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub last_sent_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub shut_up_timestamp: Option<u64>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Details {
    pub group_id: DecimalId,
    pub name: String,
    pub member_count: u64,
    pub max_member_count: u64,
    pub whole_group_muted: bool,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Owner {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub qid: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Extended {
    pub group_id: DecimalId,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Owner")]
    pub owner: Option<Owner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub robot_member_switch: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub robot_member_examine: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub robot_switch: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub custom_portrait: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub essential_message_switch: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub group_square_switch: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub lucky_word: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub lucky_word_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub viewed_message_disappear_time: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Share {
    pub group_id: DecimalId,
    pub url: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Quota {
    pub group_id: DecimalId,
    pub can_at_all: bool,
    pub remaining_for_group: u64,
    pub remaining_for_account: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutedMember {
    pub uid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub card: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub role_code: Option<i64>,
    pub shut_up_time: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Muted {
    pub group_id: DecimalId,
    pub members: Vec<MutedMember>,
    pub complete: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HonorUser {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub description: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Honors {
    pub group_id: DecimalId,
    pub kind: HonorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "HonorUser")]
    pub current_talkative: Option<HonorUser>,
    pub talkative: Vec<HonorUser>,
    pub performer: Vec<HonorUser>,
    pub legend: Vec<HonorUser>,
    pub emotion: Vec<HonorUser>,
    pub strong_newbie: Vec<HonorUser>,
    pub complete: No,
}

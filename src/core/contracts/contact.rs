//! Actual friend payloads and deliberately bounded, typed profile projections.
use super::{
    account::NumberOrString,
    group::BoolOrString,
    resource::Contact,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use crate::core::input::DecimalId;
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}
#[derive(Serialize, JsonSchema)]
pub struct UserPayload {
    pub user_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct LikePayload {
    pub user_id: String,
    pub times: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct LikesPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub user_id: Option<String>,
    pub start: NumberOrString,
    pub count: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserStatus {
    pub user_id: DecimalId,
    pub status: u64,
    pub ext_status: u64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LikeUser {
    pub uid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FavoriteInfo {
    pub total_count: u64,
    pub last_time: u64,
    pub today_count: u64,
    pub users: Vec<LikeUser>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoteInfo {
    pub total_count: u64,
    pub new_count: u64,
    pub new_nearby_count: u64,
    pub last_visit_time: u64,
    pub users: Vec<LikeUser>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Likes {
    pub user_id: DecimalId,
    pub uid: String,
    pub time: String,
    pub favorite_info: FavoriteInfo,
    pub vote_info: VoteInfo,
    pub start: u32,
    pub requested_limit: usize,
    pub complete: No,
}
#[derive(Serialize, JsonSchema)]
pub struct ProfilePayload {
    pub user_id: String,
    pub no_cache: BoolOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct RemarkPayload {
    pub user_id: String,
    pub remark: String,
}
#[derive(Serialize, JsonSchema)]
pub struct RemovePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub friend_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub user_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub temp_block: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub temp_both_del: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
pub struct RespondPayload {
    pub flag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub approve: Option<BoolOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub remark: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct DoubtPayload {
    pub flag: String,
    pub approve: bool,
}
#[derive(Serialize, JsonSchema)]
pub struct RequestsPayload {
    pub count: f64,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "contact.like")]
    Like,
    #[serde(rename = "contact.remark")]
    Remark,
    #[serde(rename = "contact.remove")]
    Remove,
    #[serde(rename = "contact.respond")]
    Respond,
    #[serde(rename = "contact.accept-doubt")]
    AcceptDoubt,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub operation: Operation,
    pub request_id: LocalId,
    pub outcome: Outcome,
    pub deduplicated: bool,
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
pub enum WriteReply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub user_id: DecimalId,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub uid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub sex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub qid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub age: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub level: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub status: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub registration_time: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub vip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub yearly_vip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "u64")]
    pub vip_level: Option<u64>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Category {
    pub category_id: u64,
    pub name: String,
    pub reported_member_count: u64,
    pub friends: Vec<Contact>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Categories {
    pub categories: Vec<Category>,
    pub filtered_by_policy: bool,
    pub complete: No,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnilateralUser {
    pub user_id: DecimalId,
    pub uid: String,
    pub nickname: String,
    pub age: u64,
    pub source: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Unilateral {
    pub users: Vec<UnilateralUser>,
    pub filtered_by_policy: bool,
    pub complete: No,
    pub upstream_batch_limit: usize,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FriendRequest {
    pub flag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "DecimalId")]
    pub user_id: Option<DecimalId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub time: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Requests {
    pub requests: Vec<FriendRequest>,
    pub filtered_by_policy: bool,
    pub complete: No,
    pub requested_limit: usize,
}

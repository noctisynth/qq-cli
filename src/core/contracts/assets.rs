//! Own-account repositories: opaque native inputs and bounded typed summaries.
use super::{
    account::NumberOrString,
    scalar::{LocalId, No, Yes},
    write::Outcome,
};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayload {
    pub raw_data: String,
    pub brief: String,
}
#[derive(Serialize, JsonSchema)]
pub struct FavoritesPayload {
    pub category: String,
    pub count: String,
}
#[derive(Serialize, JsonSchema)]
pub struct EmojiPayload {
    pub count: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Ids {
    One(String),
    Many(Vec<String>),
}
#[derive(Serialize, JsonSchema)]
pub struct RemovePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Ids")]
    pub res_id: Option<Ids>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Ids")]
    pub id: Option<Ids>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Vec<String>")]
    pub ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Ids")]
    pub md5: Option<Ids>,
}
#[derive(Serialize, JsonSchema)]
pub struct DescribePayload {
    pub emoji_id: NumberOrString,
    pub res_id: String,
    pub md5: String,
    pub desc: String,
}
#[derive(Clone, Copy, Serialize, JsonSchema)]
pub enum Operation {
    #[serde(rename = "favorite.create")]
    Create,
    #[serde(rename = "emoji.remove")]
    Remove,
    #[serde(rename = "emoji.describe")]
    Describe,
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
pub enum Reply {
    Result(super::reply::ScopedReply<Receipt>),
    Draft(super::reply::Reply<Draft>),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Favorite {
    pub id: String,
    pub kind: f64,
    pub status: f64,
    pub category: f64,
    pub create_time: String,
    pub collect_time: String,
    pub modify_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub brief: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Favorites {
    pub complete: No,
    pub body_available: No,
    pub has_more: bool,
    pub items: Vec<Favorite>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Emoji {
    pub res_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "i64")]
    pub emoji_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub md5: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub native_exists: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub animated_png: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub market_face: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Emojis {
    pub complete: No,
    pub items: Vec<Emoji>,
}

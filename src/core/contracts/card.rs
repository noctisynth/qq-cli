//! Card generation protocol, kept separate from actual message sending.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct PeerPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    pub phone_number: String,
}
#[derive(Serialize, JsonSchema)]
pub struct GroupPayload {
    pub group_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct ButtonPayload {
    pub group_id: String,
    pub bot_appid: String,
    pub button_id: String,
    pub callback_data: String,
    pub msg_seq: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Template {
    Bili,
    Weibo,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplatePayload {
    #[serde(rename = "type")]
    pub template: Template,
    pub title: String,
    pub desc: String,
    pub pic_url: String,
    pub jump_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub raw_ark_data: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CustomPayload {
    pub title: String,
    pub desc: String,
    pub pic_url: String,
    pub jump_url: String,
    pub icon_url: String,
    pub app_id: String,
    pub scene: String,
    pub template_type: String,
    pub business_type: String,
    pub ver_type: String,
    pub share_type: String,
    pub version_id: String,
    pub sdk_id: String,
    pub with_share_ticket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub web_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub raw_ark_data: Option<String>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum MiniPayload {
    Template(TemplatePayload),
    Custom(Box<CustomPayload>),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Card {
    pub conversation: crate::core::domain::Conversation,
    #[schemars(length(min = 1))]
    pub card_json: String,
}

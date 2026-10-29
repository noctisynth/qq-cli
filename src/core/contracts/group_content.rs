//! Group message administration and the upstream first sign-in page.
use super::{account::NumberOrString, scalar::No};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct GroupPayload {
    pub group_id: String,
}
#[derive(Serialize, JsonSchema)]
pub struct SignedPayload {
    pub group_id: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct DeleteEssencePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "NumberOrString")]
    pub message_id: Option<NumberOrString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub msg_seq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub msg_random: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
}
#[derive(Serialize, JsonSchema)]
pub struct TodoPayload {
    pub group_id: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub message_seq: Option<String>,
}
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoState {
    Open,
    Complete,
    Cancel,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Signer {
    pub user_id: crate::core::input::DecimalId,
    pub nickname: String,
    pub native_time: f64,
    pub rank: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Signed {
    pub group_id: crate::core::input::DecimalId,
    pub complete: No,
    pub users: Vec<Signer>,
}

#[derive(Serialize, JsonSchema)]
pub struct PublishNoticePayload {
    pub group_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub image: Option<String>,
    pub pinned: NumberOrString,
    #[serde(rename = "type")]
    pub notice_type: NumberOrString,
    pub confirm_required: NumberOrString,
    pub is_show_edit_card: NumberOrString,
    pub tip_window_type: NumberOrString,
}
#[derive(Serialize, JsonSchema)]
pub struct DeleteNoticePayload {
    pub group_id: String,
    pub notice_id: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoticeImage {
    pub id: String,
    pub width: f64,
    pub height: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Notice {
    pub notice_id: String,
    pub sender_id: crate::core::input::DecimalId,
    pub native_time: f64,
    pub text: String,
    pub images: Vec<NoticeImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "f64")]
    pub read_count: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub send_to_new_member: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "bool")]
    pub pinned: Option<bool>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Notices {
    pub group_id: crate::core::input::DecimalId,
    pub complete: No,
    pub items: Vec<Notice>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Essence {
    pub sequence: String,
    pub random: String,
    pub native_message_id: String,
    pub sender_id: crate::core::input::DecimalId,
    pub sender_nickname: String,
    pub operator_id: crate::core::input::DecimalId,
    pub operator_nickname: String,
    pub native_operator_time: f64,
    pub segments: Vec<super::message::Segment>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Essences {
    pub group_id: crate::core::input::DecimalId,
    pub complete: No,
    pub items: Vec<Essence>,
}

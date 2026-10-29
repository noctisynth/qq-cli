//! Native interaction payloads and bounded projections.
use super::{account::NumberOrString, group::BoolOrString, scalar::No};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct ReactPayload {
    pub message_id: NumberOrString,
    pub emoji_id: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "BoolOrString")]
    pub set: Option<BoolOrString>,
}
#[derive(Serialize, JsonSchema)]
pub struct FetchPayload {
    pub message_id: NumberOrString,
    #[serde(rename = "emojiId")]
    pub emoji_id: NumberOrString,
    #[serde(rename = "emojiType")]
    pub emoji_type: NumberOrString,
    pub count: NumberOrString,
    pub cookie: String,
}
#[derive(Serialize, JsonSchema)]
pub struct ListPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    pub message_id: String,
    pub emoji_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub emoji_type: Option<String>,
    pub count: f64,
}
#[derive(Serialize, JsonSchema)]
pub struct PokePayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub group_id: Option<String>,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "String")]
    pub target_id: Option<String>,
}
impl PokePayload {
    pub fn for_target(
        group_id: Option<String>,
        user_id: String,
    ) -> crate::core::error::Result<Self> {
        if !crate::core::domain::is_exact_js_id(&user_id)
            || group_id
                .as_deref()
                .is_some_and(|id| !crate::core::domain::is_exact_js_id(id))
        {
            return Err(crate::core::error::AppError::new(
                "POKE_ID_RANGE",
                "Poke identifiers must be exact positive JavaScript integers.",
                2,
            ));
        }
        Ok(Self {
            group_id,
            user_id,
            target_id: None,
        })
    }
}
#[derive(Serialize, JsonSchema)]
pub struct InputPayload {
    pub user_id: String,
    pub event_type: f64,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reactor {
    pub reactor_id: String,
    pub nickname: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reactions {
    pub conversation: crate::core::domain::Conversation,
    pub emoji_id: String,
    pub complete: No,
    pub items: Vec<Reactor>,
}

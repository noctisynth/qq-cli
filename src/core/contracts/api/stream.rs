//! Explicit raw stream records are the API-only dynamic result boundary.
use super::{Draft, Effect, json::Document};
use crate::core::contracts::{scalar::Yes, write::Outcome};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    Ready {
        action: String,
        effect: Effect,
        identity_verified: Yes,
    },
    Packet {
        action: String,
        index: usize,
        provisional: Yes,
        result: Document,
    },
    Summary {
        action: String,
        outcome: Outcome,
        deduplicated: bool,
        records: usize,
        packets_available: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        transfer_complete: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error_code: Option<&'static str>,
    },
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply {
    Stream(crate::core::contracts::reply::ScopedReply<Event>),
    Draft(crate::core::contracts::reply::Reply<Draft>),
}

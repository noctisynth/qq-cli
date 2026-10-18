//! Dedicated bounded OneBot media bytes; local paths and URLs are never dereferenced.
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::io::Read;
use std::time::Instant;

use crate::client::{Action, OneBotClient};
use crate::error::{AppError, Result};
use crate::media::{MAX_BASE64_BYTES, MAX_DOWNLOAD_BYTES};

pub struct Bytes {
    pub encoded: String,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(borrow)]
    status: Cow<'a, str>,
    retcode: i64,
    #[serde(borrow)]
    data: Option<Data<'a>>,
}
#[derive(Deserialize)]
struct Data<'a> {
    #[serde(borrow)]
    base64: Option<Cow<'a, str>>,
    #[serde(borrow)]
    file_size: Option<Cow<'a, str>>,
}

pub(crate) fn fetch(
    client: &OneBotClient,
    action: Action,
    payload: &Value,
    deadline: Instant,
) -> Result<Bytes> {
    if !matches!(action, Action::File | Action::Image) {
        return Err(AppError::invalid());
    }
    let limit = MAX_BASE64_BYTES + client.response_limit();
    let response = client.response(action, payload, deadline)?;
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(limit_error());
    }
    let mut body = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|_| AppError::new("BACKEND_TRANSPORT", "Media response was interrupted.", 3))?;
    if body.len() > limit {
        return Err(limit_error());
    }
    from_slice(&body, client.response_limit())
}

pub(crate) fn from_slice(body: &[u8], metadata_limit: usize) -> Result<Bytes> {
    if body.len() > MAX_BASE64_BYTES + metadata_limit {
        return Err(limit_error());
    }
    // Typed deserialization ignores path/URL/unknown objects rather than allocating
    // a large arbitrary Value tree. Unescaped base64 borrows the wire buffer.
    let envelope: Envelope<'_> = serde_json::from_slice(body).map_err(|_| AppError::protocol())?;
    crate::client::final_status(&envelope.status, envelope.retcode)?;
    let data = envelope.data.ok_or_else(AppError::protocol)?;
    let encoded = data.base64.ok_or_else(unavailable)?;
    if encoded.len() > MAX_BASE64_BYTES || body.len().saturating_sub(encoded.len()) > metadata_limit
    {
        return Err(limit_error());
    }
    let size = size(data.file_size.as_deref())?;
    Ok(Bytes {
        encoded: encoded.into_owned(),
        size,
    })
}

pub fn from_value(mut value: Value) -> Result<Bytes> {
    let encoded = match value.get_mut("base64").map(Value::take) {
        Some(Value::String(value)) => value,
        _ => return Err(unavailable()),
    };
    if encoded.len() > MAX_BASE64_BYTES
        || serde_json::to_vec(&value)
            .map_err(|_| AppError::protocol())?
            .len()
            > 4 * 1024 * 1024
    {
        return Err(limit_error());
    }
    let size = match value.get("file_size") {
        None | Some(Value::Null) => None,
        Some(Value::String(value)) => size(Some(value))?,
        _ => return Err(AppError::protocol()),
    };
    Ok(Bytes { encoded, size })
}
fn size(value: Option<&str>) -> Result<Option<u64>> {
    match value {
        None | Some("") => Ok(None),
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| *value <= MAX_DOWNLOAD_BYTES)
            .map(Some)
            .ok_or_else(AppError::protocol),
    }
}
fn unavailable() -> AppError {
    let mut error = AppError::new(
        "MEDIA_BYTES_UNAVAILABLE",
        "Backend did not provide media bytes; inspect its managed media capability.",
        4,
    );
    error.next_action = Some(Box::new(serde_json::json!({"command":"doctor","args":{}})));
    error
}
fn limit_error() -> AppError {
    AppError::new(
        "RESPONSE_LIMIT",
        "Media response exceeded its dedicated byte or metadata budget.",
        3,
    )
}

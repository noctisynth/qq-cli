//! Validate cached message receipts before reporting acceptance from disk.
use crate::domain::{Conversation, identifier};
use crate::error::{AppError, Result};
use serde_json::Value;

pub fn validate(value: &Value, expected: Option<&Conversation>) -> Result<()> {
    let fields = value.as_object().ok_or_else(invalid)?;
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "messageId" | "conversation" | "attachments" | "fileTransfer"
        )
    }) {
        return Err(invalid());
    }
    let id = value
        .get("messageId")
        .filter(|v| v.is_string())
        .and_then(message_id)
        .ok_or_else(invalid)?;
    if id.is_empty() {
        return Err(invalid());
    }
    let target = value
        .get("conversation")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?
        .parse::<Conversation>()
        .map_err(|_| invalid())?;
    if expected.is_some_and(|expected| expected != &target) {
        return Err(invalid());
    }
    match value.get("attachments") {
        None if value.get("fileTransfer").is_none() => (),
        Some(Value::Array(files))
            if !files.is_empty()
                && files.len() <= 16
                && value["fileTransfer"] == "not_observable" =>
        {
            for file in files {
                attachment(file)?;
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}

pub fn message_id(value: &Value) -> Option<String> {
    let id = identifier(value)?;
    let canonical = if id.starts_with('-') {
        let value = id.parse::<i64>().ok()?;
        if value == 0 {
            return None;
        }
        value.to_string()
    } else {
        let value = id.parse::<u64>().ok()?;
        if value == 0 {
            return None;
        }
        value.to_string()
    };
    (id == canonical).then_some(id)
}

pub fn attachment(value: &Value) -> Result<()> {
    let fields = value.as_object().ok_or_else(invalid)?;
    if fields.len() != 4 || !matches!(value["kind"].as_str(), Some("image" | "file")) {
        return Err(invalid());
    }
    let name = value["name"].as_str().ok_or_else(invalid)?;
    if std::path::Path::new(name).components().count() != 1 {
        return Err(invalid());
    }
    crate::archive::member_path(name).map_err(|_| invalid())?;
    value["sizeBytes"]
        .as_u64()
        .filter(|size| *size > 0 && *size <= crate::media::MAX_DOWNLOAD_BYTES)
        .ok_or_else(invalid)?;
    let hash = value["sha256"].as_str().ok_or_else(invalid)?;
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> AppError {
    AppError::new(
        "REQUEST_RECEIPT_INVALID",
        "Stored message receipt is invalid; inspect the request state without sending again.",
        6,
    )
}

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Identifiers remain decimal strings throughout the application.
pub fn decimal_id(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
        || value.len() > 32
    {
        return Err("Expected a positive decimal identifier".into());
    }
    Ok(value.to_owned())
}

pub fn request_key(value: &str) -> Result<String, String> {
    if !(1..=128).contains(&value.len())
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err("Expected 1 to 128 ASCII letters, digits, hyphens or underscores".into());
    }
    Ok(value.to_owned())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Conversation {
    Private(String),
    Group(String),
}

impl Conversation {
    pub fn id(&self) -> &str {
        match self {
            Self::Private(id) | Self::Group(id) => id,
        }
    }
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(_))
    }
}

impl fmt::Display for Conversation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Private(id) => write!(f, "private:{id}"),
            Self::Group(id) => write!(f, "group:{id}"),
        }
    }
}

impl FromStr for Conversation {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (kind, id) = value
            .split_once(':')
            .ok_or("Expected private:ID or group:ID")?;
        let id = decimal_id(id)?;
        match kind {
            "private" => Ok(Self::Private(id)),
            "group" => Ok(Self::Group(id)),
            _ => Err("Expected private:ID or group:ID".into()),
        }
    }
}

impl TryFrom<String> for Conversation {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Conversation> for String {
    fn from(value: Conversation) -> Self {
        value.to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub instance_id: String,
    pub account: String,
}

/// Convert JSON integers without passing through floating point.
pub fn identifier(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => {
            let digits = s.strip_prefix('-').unwrap_or(s);
            (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then(|| s.clone())
        }
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(|n| n.to_string())
            .or_else(|| n.as_u64().map(|n| n.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_never_rounds_and_conversation_is_explicit() {
        let value: serde_json::Value = serde_json::from_str("18446744073709551615").unwrap();
        assert_eq!(identifier(&value).as_deref(), Some("18446744073709551615"));
        assert_eq!(identifier(&serde_json::json!(1.25)), None);
        for bad in ["group:0", "private:01", "10001", "other:10001"] {
            assert!(bad.parse::<Conversation>().is_err());
        }
        let peer: Conversation = "group:10001".parse().unwrap();
        assert_eq!(serde_json::to_string(&peer).unwrap(), "\"group:10001\"");
    }
}

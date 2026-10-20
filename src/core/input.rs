//! Shared validated input scalars. Clap parses these same types that publish schemas.
use schemars::JsonSchema;
use serde::Serialize;
use std::{fmt, ops::Deref, str::FromStr};
pub const ATTACHMENT_LIMIT: usize = 16;
pub const MENTION_LIMIT: usize = 64;
pub const MAX_TEXT_BYTES: usize = 65536;
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Bounded<const MAX: u16>(#[schemars(range(min=1,max=MAX))] u16);
impl<const MAX: u16> Bounded<MAX> {
    pub const fn new(value: u16) -> Self {
        assert!(value > 0 && value <= MAX);
        Self(value)
    }
    pub fn get(self) -> u16 {
        self.0
    }
}
impl<const MAX: u16> fmt::Display for Bounded<MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<const MAX: u16> FromStr for Bounded<MAX> {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        let value = value
            .parse::<u16>()
            .map_err(|_| "Expected a positive integer".to_owned())?;
        if value == 0 || value > MAX {
            return Err(format!("Expected an integer between 1 and {MAX}"));
        }
        Ok(Self(value))
    }
}
impl<const MAX: u16> From<u16> for Bounded<MAX> {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}
impl<const MAX: u16> From<Bounded<MAX>> for u16 {
    fn from(value: Bounded<MAX>) -> Self {
        value.0
    }
}
impl<const MAX: u16> From<Bounded<MAX>> for u64 {
    fn from(value: Bounded<MAX>) -> Self {
        value.0.into()
    }
}
impl<const MAX: u16> From<Bounded<MAX>> for usize {
    fn from(value: Bounded<MAX>) -> Self {
        value.0.into()
    }
}
pub type Limit = Bounded<1000>;
pub type Seconds = Bounded<300>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct DecimalId(#[schemars(regex(pattern = "^[1-9][0-9]{0,31}$"))] String);
impl FromStr for DecimalId {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        super::domain::decimal_id(value).map(Self)
    }
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Text(#[schemars(length(min=1),extend("maxUtf8Bytes"=MAX_TEXT_BYTES))] String);
impl FromStr for Text {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        if value.is_empty() || value.len() > MAX_TEXT_BYTES {
            return Err(format!("Expected 1 to {MAX_TEXT_BYTES} UTF-8 bytes"));
        }
        Ok(Self(value.into()))
    }
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Rfc3339(#[schemars(extend("format"="date-time"))] String);
impl FromStr for Rfc3339 {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        chrono::DateTime::parse_from_rfc3339(value)
            .map_err(|_| "Expected an RFC3339 timestamp".to_owned())?;
        Ok(Self(value.into()))
    }
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Handle(#[schemars(length(min = 1, max = 128))] String);
impl FromStr for Handle {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        if value.is_empty() || value.len() > 128 {
            return Err("Expected a reference of 1 to 128 bytes".into());
        }
        Ok(Self(value.into()))
    }
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct NonEmpty(#[schemars(length(min = 1))] String);
impl FromStr for NonEmpty {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, String> {
        if value.is_empty() {
            return Err("Expected nonempty text".into());
        }
        Ok(Self(value.into()))
    }
}
macro_rules! string_access {($($ty:ty),+)=>{$(
 impl Deref for $ty {type Target=str;fn deref(&self)->&str {&self.0}}
 impl fmt::Display for $ty {fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {self.0.fmt(f)}}
 )+};}
string_access!(DecimalId, Text, Rfc3339, Handle, NonEmpty);

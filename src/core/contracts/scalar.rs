//! Small wire invariants; constants are shared by serialization and schema attributes.
use crate::core::{
    domain::request_key,
    error::{AppError, Result},
};
use schemars::JsonSchema;
use serde::{Serialize, Serializer};

#[derive(Clone, Copy, JsonSchema)]
#[schemars(with = "u8", extend("const" = Self::VALUE))]
pub struct WireVersion;
impl WireVersion {
    pub const VALUE: u8 = 1;
}
impl Serialize for WireVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u8(Self::VALUE)
    }
}
#[derive(Clone, Copy, JsonSchema)]
#[schemars(with = "bool", extend("const" = Self::VALUE))]
pub struct Yes;
impl Yes {
    pub const VALUE: bool = true;
}
impl Serialize for Yes {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_bool(Self::VALUE)
    }
}
#[derive(Clone, Copy, JsonSchema)]
#[schemars(with = "bool", extend("const" = Self::VALUE))]
pub struct No;
impl No {
    pub const VALUE: bool = false;
}
impl Serialize for No {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_bool(Self::VALUE)
    }
}
#[derive(Clone, Debug, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct LocalId(
    #[schemars(length(min = 1, max = 128), regex(pattern = "^[A-Za-z0-9_-]+$"))] String,
);
impl LocalId {
    pub fn new(value: &str) -> Result<Self> {
        request_key(value)
            .map(Self)
            .map_err(|_| AppError::storage())
    }
}

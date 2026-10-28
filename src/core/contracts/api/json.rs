//! The only public dynamic result boundary: explicitly requested upstream API JSON.
use crate::core::error::{AppError, Result};
use schemars::JsonSchema;
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor},
};
use serde_json::{Value, value::RawValue};
use std::fmt;

#[derive(Serialize, JsonSchema)]
#[serde(transparent)]
pub struct Document(#[schemars(with = "serde_json::Value")] Value);
impl Document {
    pub fn new(value: Value) -> Self {
        Self(value)
    }
}
pub const MAX_INPUT: usize = 1024 * 1024;
pub fn parameters(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_INPUT {
        return Err(invalid());
    }
    let raw: Box<RawValue> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if !raw.get().starts_with('{') {
        return Err(invalid());
    }
    validate(&raw, 0)?;
    serde_json::from_str(raw.get()).map_err(|_| invalid())
}
fn validate(raw: &RawValue, depth: usize) -> Result<()> {
    if depth >= 64 {
        return Err(invalid());
    }
    match raw.get().as_bytes().first() {
        Some(b'{') => {
            let object: UniqueObject = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
            for value in object.0 {
                validate(&value, depth + 1)?;
            }
        }
        Some(b'[') => {
            let array: Vec<Box<RawValue>> =
                serde_json::from_str(raw.get()).map_err(|_| invalid())?;
            for value in array {
                validate(&value, depth + 1)?;
            }
        }
        Some(b'-' | b'0'..=b'9')
            if raw.get().len() > 512 || !raw.get().parse::<f64>().is_ok_and(f64::is_finite) =>
        {
            return Err(invalid());
        }
        _ => (),
    }
    Ok(())
}
struct UniqueObject(Vec<Box<RawValue>>);
impl<'de> Deserialize<'de> for UniqueObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = UniqueObject;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an object without duplicate keys")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut keys = std::collections::BTreeSet::new();
                let mut values = Vec::new();
                while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if !keys.insert(key) {
                        return Err(serde::de::Error::custom("duplicate key"));
                    }
                    values.push(value);
                }
                Ok(UniqueObject(values))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}
fn invalid() -> AppError {
    AppError::new(
        "API_PARAMS_INVALID",
        "Expected a bounded JSON object without duplicate keys or excessive nesting.",
        2,
    )
}

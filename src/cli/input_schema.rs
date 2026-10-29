//! Parser type discovery, independent of argument names.
use crate::core::{
    contracts::{scalar::LocalId, schema},
    domain::Conversation,
    input::{DecimalId, Handle, Limit, NonEmpty, Rfc3339, Seconds, StartupSeconds, Text},
};
use clap::{Arg, ArgAction};
use serde_json::{Value, json};
use std::any::TypeId;
fn generated<T: schemars::JsonSchema>() -> Value {
    let mut value = serde_json::to_value(schema::<T>()).expect("schema serializes");
    if let Some(object) = value.as_object_mut() {
        object.remove("$schema");
        object.remove("title");
    }
    value
}
pub fn property(arg: &Arg) -> crate::core::error::Result<Value> {
    let parser = arg.get_value_parser();
    let id = parser.type_id();
    let mut value = if matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse) {
        generated::<bool>()
    } else if id == TypeId::of::<crate::core::input::ActionName>() {
        generated::<crate::core::input::ActionName>()
    } else if id == TypeId::of::<crate::core::input::TextValue>() {
        generated::<crate::core::input::TextValue>()
    } else if id == TypeId::of::<bool>() {
        generated::<bool>()
    } else if id == TypeId::of::<crate::core::contracts::group_query::HonorKind>() {
        generated::<crate::core::contracts::group_query::HonorKind>()
    } else if id == TypeId::of::<crate::core::contracts::group::InvitePolicy>() {
        generated::<crate::core::contracts::group::InvitePolicy>()
    } else if id == TypeId::of::<u32>() {
        generated::<u32>()
    } else if id == TypeId::of::<LocalId>() {
        generated::<LocalId>()
    } else if id == TypeId::of::<DecimalId>() {
        generated::<DecimalId>()
    } else if id == TypeId::of::<Conversation>() {
        generated::<Conversation>()
    } else if id == TypeId::of::<Handle>() {
        generated::<Handle>()
    } else if id == TypeId::of::<NonEmpty>() {
        generated::<NonEmpty>()
    } else if id == TypeId::of::<Text>() {
        generated::<Text>()
    } else if id == TypeId::of::<Rfc3339>() {
        generated::<Rfc3339>()
    } else if id == TypeId::of::<Limit>() {
        generated::<Limit>()
    } else if id == TypeId::of::<Seconds>() {
        generated::<Seconds>()
    } else if id == TypeId::of::<StartupSeconds>() {
        generated::<StartupSeconds>()
    } else if id == TypeId::of::<std::path::PathBuf>() {
        generated::<std::path::PathBuf>()
    } else if id == TypeId::of::<String>() {
        generated::<String>()
    } else {
        return Err(crate::core::error::AppError::new(
            "INPUT_TYPE_UNREGISTERED",
            "Input parser type has no registered schema adapter.",
            6,
        ));
    };
    if value["type"] == "string"
        && let Some(values) = parser.possible_values()
    {
        let choices = values
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_owned())
            .collect::<Vec<_>>();
        if !choices.is_empty() {
            value["enum"] = json!(choices);
        }
    }
    if matches!(arg.get_action(), ArgAction::Append) {
        value = json!({"type":"array","items":value});
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renamed_arguments_keep_parser_constraints_without_name_special_cases() {
        let arg = Arg::new("arbitrary_name").value_parser(clap::value_parser!(Limit));
        let schema = property(&arg).unwrap();
        assert_eq!(schema["minimum"], 1);
        assert_eq!(schema["maximum"], 1000);
        let arg = Arg::new("another_name").value_parser(clap::value_parser!(LocalId));
        assert_eq!(property(&arg).unwrap()["pattern"], "^[A-Za-z0-9_-]+$");
    }
}

//! Public wire contracts. Business serialization and schema generation use these same types.
pub mod installation;
pub mod reply;
pub mod scalar;

use schemars::{JsonSchema, Schema, generate::SchemaSettings};
pub fn schema<T: JsonSchema>() -> Schema {
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
}

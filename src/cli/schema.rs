//! Generated output schemas; definitions live with actual serialized types.
use serde_json::Value;
pub fn error_envelope() -> Value {
    serde_json::to_value(crate::core::contracts::schema::<
        crate::core::contracts::reply::Failure,
    >())
    .expect("schema serializes")
}
pub fn event_stream() -> Value {
    serde_json::to_value(crate::core::contracts::schema::<
        crate::core::contracts::event::Event,
    >())
    .expect("schema serializes")
}

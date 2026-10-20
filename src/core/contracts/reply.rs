//! The actual typed response container, including the actual error serializer.
use super::scalar::{No, WireVersion, Yes};
use crate::core::{domain::Scope, error::AppError};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub scope: Scope,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Success<T> {
    schema_version: WireVersion,
    ok: Yes,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Meta")]
    pub meta: Option<Meta>,
}
impl<T> Success<T> {
    pub fn new(data: T) -> Self {
        Self {
            schema_version: WireVersion,
            ok: Yes,
            data,
            meta: None,
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failure {
    schema_version: WireVersion,
    ok: No,
    pub error: AppError,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Meta")]
    pub meta: Option<Meta>,
}
impl Failure {
    pub fn new(mut error: AppError) -> Self {
        let meta = error.scope.take().map(|scope| Meta { scope: *scope });
        Self {
            schema_version: WireVersion,
            ok: No,
            error,
            meta,
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Reply<T> {
    Success(Success<T>),
    Failure(Failure),
}

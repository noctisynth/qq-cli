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
pub struct Success<T, M = OptionalScope> {
    schema_version: WireVersion,
    ok: Yes,
    pub data: T,
    #[serde(flatten)]
    pub metadata: M,
}
impl<T> Success<T> {
    pub fn new(data: T) -> Self {
        Self {
            schema_version: WireVersion,
            ok: Yes,
            data,
            metadata: OptionalScope { meta: None },
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptionalScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Meta")]
    pub meta: Option<Meta>,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scoped {
    pub meta: Meta,
}
impl<T> Success<T, Scoped> {
    pub fn scoped(data: T, scope: Scope) -> Self {
        Self {
            schema_version: WireVersion,
            ok: Yes,
            data,
            metadata: Scoped {
                meta: Meta { scope },
            },
        }
    }
}
pub type ScopedReply<T> = Reply<T, Scoped>;

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
pub enum Reply<T, M = OptionalScope> {
    Success(Success<T, M>),
    Failure(Failure),
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Paged<T, P> {
    #[serde(flatten)]
    pub reply: Success<T, Scoped>,
    pub page_info: P,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum PagedReply<T, P> {
    Success(Paged<T, P>),
    Failure(Failure),
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Partial<T, P> {
    schema_version: WireVersion,
    ok: No,
    pub data: T,
    pub error: AppError,
    pub meta: Meta,
    pub page_info: P,
}
impl<T, P> Partial<T, P> {
    pub fn new(data: T, page_info: P, scope: Scope) -> Self {
        Self {
            schema_version: WireVersion,
            ok: No,
            data,
            error: AppError::new(
                "PARTIAL_RESULT",
                "The requested page was interrupted; inspect pageInfo before continuing.",
                7,
            ),
            meta: Meta { scope },
            page_info,
        }
    }
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum PageReply<T, P> {
    Success(Paged<T, P>),
    Partial(Partial<T, P>),
    Failure(Failure),
}

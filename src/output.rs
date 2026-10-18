use serde::Serialize;
use serde_json::Value;

use crate::domain::Scope;
use crate::error::AppError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    schema_version: u8,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<AppError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_info: Option<Value>,
    #[serde(skip)]
    exit_code: u8,
}

#[derive(Serialize)]
struct Meta {
    scope: Scope,
}

impl Envelope {
    pub fn success(data: Value) -> Self {
        Self {
            schema_version: 1,
            ok: true,
            data: Some(data),
            error: None,
            meta: None,
            page_info: None,
            exit_code: 0,
        }
    }
    pub fn failure(mut error: AppError) -> Self {
        let exit_code = error.exit_code;
        let meta = error.scope.take().map(|scope| Meta { scope: *scope });
        Self {
            schema_version: 1,
            ok: false,
            data: None,
            error: Some(error),
            meta,
            page_info: None,
            exit_code,
        }
    }
    pub fn scoped(mut self, scope: Scope) -> Self {
        self.meta = Some(Meta { scope });
        self
    }
    pub fn paged(mut self, page_info: Value) -> Self {
        self.page_info = Some(page_info);
        self
    }
    pub fn partial(data: Value, page_info: Value, scope: Scope) -> Self {
        let mut result = Self::failure(AppError::new(
            "PARTIAL_RESULT",
            "The requested page was interrupted; inspect pageInfo before continuing.",
            7,
        ));
        result.data = Some(data);
        result.page_info = Some(page_info);
        result.meta = Some(Meta { scope });
        result
    }
    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

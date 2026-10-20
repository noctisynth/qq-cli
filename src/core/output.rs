//! Response dispatch. Legacy JSON paths are temporary until TASK-004 completes.
use crate::core::contracts::{
    installation::{Installation, Installed, RecordedInstance, Uninstalled},
    reply::{Failure, Meta, Reply, Success},
};
use crate::core::{domain::Scope, error::AppError};
use serde::Serialize;
use serde_json::Value;

/// Closed payload dispatch for migrated commands. Never add a Value fallback here.
#[derive(Serialize)]
#[serde(untagged)]
pub enum PublicData {
    Installed(Installed),
    Uninstalled(Uninstalled),
    Installations(Vec<Installation>),
    Instances(Vec<RecordedInstance>),
}
impl From<Installed> for PublicData {
    fn from(value: Installed) -> Self {
        Self::Installed(value)
    }
}
impl From<Uninstalled> for PublicData {
    fn from(value: Uninstalled) -> Self {
        Self::Uninstalled(value)
    }
}
impl From<Vec<Installation>> for PublicData {
    fn from(value: Vec<Installation>) -> Self {
        Self::Installations(value)
    }
}
impl From<Vec<RecordedInstance>> for PublicData {
    fn from(value: Vec<RecordedInstance>) -> Self {
        Self::Instances(value)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum Wire {
    Legacy(LegacyWire),
    Typed(Reply<PublicData>),
}
/// Explicit migration-only body for commands not yet converted. Remove under TASK-004.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWire {
    schema_version: u8,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<AppError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
}
#[derive(Serialize)]
pub struct Envelope {
    #[serde(flatten)]
    wire: Wire,
    // Migration-only paging payload; typed paging is a later TASK-004 slice.
    #[serde(rename = "pageInfo", skip_serializing_if = "Option::is_none")]
    page_info: Option<Value>,
    #[serde(skip)]
    exit_code: u8,
}
impl Envelope {
    pub fn typed(data: impl Into<PublicData>) -> Self {
        Self {
            wire: Wire::Typed(Reply::Success(Success::new(data.into()))),
            page_info: None,
            exit_code: 0,
        }
    }
    /// Legacy entry, prohibited in migrated command slices.
    pub fn success(data: Value) -> Self {
        Self {
            wire: Wire::Legacy(LegacyWire {
                schema_version: 1,
                ok: true,
                data: Some(data),
                error: None,
                meta: None,
            }),
            page_info: None,
            exit_code: 0,
        }
    }
    pub fn failure(error: AppError) -> Self {
        let exit_code = error.exit_code;
        Self {
            wire: Wire::Typed(Reply::Failure(Failure::new(error))),
            page_info: None,
            exit_code,
        }
    }
    pub fn scoped(mut self, scope: Scope) -> Self {
        let meta = Some(Meta { scope });
        match &mut self.wire {
            Wire::Legacy(w) => w.meta = meta,
            Wire::Typed(Reply::Success(w)) => w.meta = meta,
            Wire::Typed(Reply::Failure(w)) => w.meta = meta,
        }
        self
    }
    pub fn paged(mut self, page_info: Value) -> Self {
        self.page_info = Some(page_info);
        self
    }
    pub fn partial(data: Value, page_info: Value, scope: Scope) -> Self {
        let error = AppError::new(
            "PARTIAL_RESULT",
            "The requested page was interrupted; inspect pageInfo before continuing.",
            7,
        );
        Self {
            wire: Wire::Legacy(LegacyWire {
                schema_version: 1,
                ok: false,
                data: Some(data),
                error: Some(error),
                meta: Some(Meta { scope }),
            }),
            page_info: Some(page_info),
            exit_code: 7,
        }
    }
    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

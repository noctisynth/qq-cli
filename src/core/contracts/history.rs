//! Bounded history pagination and export results.
use super::scalar::{No, Yes};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    BackendAccessible,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    NewestFirst,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Counts {
    pub scanned: usize,
    pub backend_pages: usize,
    pub coverage: Coverage,
    pub order: Order,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum End {
    BackendEnd,
    RangeSatisfied,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Interrupted {
    Limit,
    Deadline,
    ScanLimit,
    PageLimit,
    ByteLimit,
    BackendError,
    NoProgress,
}
#[derive(Debug, Serialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
pub enum HistoryPage {
    End {
        complete: Yes,
        #[serde(rename = "stopReason")]
        stop_reason: End,
        #[serde(flatten)]
        counts: Counts,
    },
    More {
        complete: No,
        #[serde(rename = "stopReason")]
        stop_reason: Interrupted,
        #[serde(flatten)]
        counts: Counts,
        #[serde(rename = "nextCursor")]
        #[schemars(regex(pattern = "^h1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$"))]
        next_cursor: String,
        #[serde(rename = "interruptionCode", skip_serializing_if = "Option::is_none")]
        #[schemars(with = "String")]
        interruption_code: Option<String>,
    },
}
impl HistoryPage {
    pub fn new(
        scanned: usize,
        backend_pages: usize,
        reason: &str,
        next: Option<String>,
        code: Option<String>,
    ) -> crate::core::error::Result<Self> {
        let counts = Counts {
            scanned,
            backend_pages,
            coverage: Coverage::BackendAccessible,
            order: Order::NewestFirst,
        };
        Ok(if let Some(next_cursor) = next {
            Self::More {
                complete: No,
                stop_reason: match reason {
                    "limit" => Interrupted::Limit,
                    "deadline" => Interrupted::Deadline,
                    "scan_limit" => Interrupted::ScanLimit,
                    "page_limit" => Interrupted::PageLimit,
                    "byte_limit" => Interrupted::ByteLimit,
                    "backend_error" => Interrupted::BackendError,
                    "no_progress" => Interrupted::NoProgress,
                    _ => return Err(crate::core::error::AppError::protocol()),
                },
                counts,
                next_cursor,
                interruption_code: code,
            }
        } else {
            Self::End {
                complete: Yes,
                stop_reason: match reason {
                    "backend_end" => End::BackendEnd,
                    "range_satisfied" => End::RangeSatisfied,
                    _ => return Err(crate::core::error::AppError::protocol()),
                },
                counts,
            }
        })
    }
    pub fn complete(&self) -> bool {
        matches!(self, Self::End { .. })
    }
    pub fn cursor(&self) -> Option<&str> {
        match self {
            Self::More { next_cursor, .. } => Some(next_cursor),
            _ => None,
        }
    }
    pub fn scanned(&self) -> usize {
        match self {
            Self::End { counts, .. } | Self::More { counts, .. } => counts.scanned,
        }
    }
}
#[derive(Serialize, JsonSchema)]
pub enum Format {
    #[serde(rename = "qq-cli-ndjson-v1")]
    NdjsonV1,
}
#[derive(Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Export {
    pub output: std::path::PathBuf,
    pub count: u64,
    pub added: usize,
    pub complete: bool,
    pub format: Format,
}

#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum ExportReply {
    Finished(super::reply::ScopedReply<Export>),
    Page(super::reply::PageReply<Export, HistoryPage>),
}

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
    Accepted(crate::core::contracts::write::Accepted),
    DryRun(crate::core::contracts::write::DryRun),
    MessageRequest(crate::core::contracts::write::MessageRequest),
    StartRequest(crate::core::contracts::write::StartRequest),
    Download(crate::core::contracts::write::Download),
    Pruned(crate::core::contracts::write::Pruned),
    Export(crate::core::contracts::history::Export),
    Message(crate::core::contracts::message::Message),
    Messages(Vec<crate::core::contracts::message::Message>),
    Resource(crate::core::contracts::resource::Row),
    Resources(Vec<crate::core::contracts::resource::Row>),
    Diagnostic(crate::core::contracts::diagnostic::Diagnostic),
    Observed(crate::core::contracts::instance::Observed),
    Pending(crate::core::contracts::instance::Pending),
    Started(crate::core::contracts::instance::Started),
    Stopped(crate::core::contracts::instance::Stopped),
    Removed(crate::core::contracts::instance::Removed),
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
    History(
        crate::core::contracts::reply::PageReply<
            PublicData,
            crate::core::contracts::history::HistoryPage,
        >,
    ),
    Legacy(LegacyWire),
    Typed(Reply<PublicData>),
    Collection(
        crate::core::contracts::reply::Paged<
            PublicData,
            crate::core::contracts::resource::CollectionPage,
        >,
    ),
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
    pub fn collection(
        data: Vec<crate::core::contracts::resource::Row>,
        scope: Scope,
        page_info: crate::core::contracts::resource::CollectionPage,
    ) -> Self {
        let mut reply = Success::new(PublicData::Resources(data));
        reply.meta = Some(Meta { scope });
        Self {
            wire: Wire::Collection(crate::core::contracts::reply::Paged { reply, page_info }),
            page_info: None,
            exit_code: 0,
        }
    }
    pub fn history(
        data: impl Into<PublicData>,
        scope: Scope,
        page_info: crate::core::contracts::history::HistoryPage,
        partial: bool,
    ) -> Self {
        use crate::core::contracts::reply::{PageReply, Paged, Partial};
        let wire = if partial {
            PageReply::Partial(Partial::new(data.into(), page_info, scope))
        } else {
            let mut reply = Success::new(data.into());
            reply.meta = Some(Meta { scope });
            PageReply::Success(Paged { reply, page_info })
        };
        Self {
            wire: Wire::History(wire),
            page_info: None,
            exit_code: if partial { 7 } else { 0 },
        }
    }
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
        let meta = Meta { scope };
        match &mut self.wire {
            Wire::History(w) => match w {
                crate::core::contracts::reply::PageReply::Success(w) => w.reply.meta = Some(meta),
                crate::core::contracts::reply::PageReply::Partial(w) => w.meta = meta,
                crate::core::contracts::reply::PageReply::Failure(w) => w.meta = Some(meta),
            },
            Wire::Legacy(w) => w.meta = Some(meta),
            Wire::Collection(w) => w.reply.meta = Some(meta),
            Wire::Typed(Reply::Success(w)) => w.meta = Some(meta),
            Wire::Typed(Reply::Failure(w)) => w.meta = Some(meta),
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

impl From<crate::core::contracts::instance::Observed> for PublicData {
    fn from(value: crate::core::contracts::instance::Observed) -> Self {
        Self::Observed(value)
    }
}
impl From<crate::core::contracts::instance::Pending> for PublicData {
    fn from(value: crate::core::contracts::instance::Pending) -> Self {
        Self::Pending(value)
    }
}
impl From<crate::core::contracts::instance::Started> for PublicData {
    fn from(value: crate::core::contracts::instance::Started) -> Self {
        Self::Started(value)
    }
}
impl From<crate::core::contracts::instance::Stopped> for PublicData {
    fn from(value: crate::core::contracts::instance::Stopped) -> Self {
        Self::Stopped(value)
    }
}
impl From<crate::core::contracts::instance::Removed> for PublicData {
    fn from(value: crate::core::contracts::instance::Removed) -> Self {
        Self::Removed(value)
    }
}

impl From<crate::core::contracts::diagnostic::Diagnostic> for PublicData {
    fn from(value: crate::core::contracts::diagnostic::Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<crate::core::contracts::resource::Row> for PublicData {
    fn from(value: crate::core::contracts::resource::Row) -> Self {
        Self::Resource(value)
    }
}

impl From<crate::core::contracts::message::Message> for PublicData {
    fn from(value: crate::core::contracts::message::Message) -> Self {
        Self::Message(value)
    }
}

impl From<Vec<crate::core::contracts::message::Message>> for PublicData {
    fn from(value: Vec<crate::core::contracts::message::Message>) -> Self {
        Self::Messages(value)
    }
}
impl From<crate::core::contracts::history::Export> for PublicData {
    fn from(value: crate::core::contracts::history::Export) -> Self {
        Self::Export(value)
    }
}

impl From<crate::core::contracts::write::Accepted> for PublicData {
    fn from(value: crate::core::contracts::write::Accepted) -> Self {
        Self::Accepted(value)
    }
}

impl From<crate::core::contracts::write::DryRun> for PublicData {
    fn from(value: crate::core::contracts::write::DryRun) -> Self {
        Self::DryRun(value)
    }
}

impl From<crate::core::contracts::write::MessageRequest> for PublicData {
    fn from(value: crate::core::contracts::write::MessageRequest) -> Self {
        Self::MessageRequest(value)
    }
}

impl From<crate::core::contracts::write::StartRequest> for PublicData {
    fn from(value: crate::core::contracts::write::StartRequest) -> Self {
        Self::StartRequest(value)
    }
}

impl From<crate::core::contracts::write::Download> for PublicData {
    fn from(value: crate::core::contracts::write::Download) -> Self {
        Self::Download(value)
    }
}

impl From<crate::core::contracts::write::Pruned> for PublicData {
    fn from(value: crate::core::contracts::write::Pruned) -> Self {
        Self::Pruned(value)
    }
}

//! Closed response dispatch over actual serialized contracts.
use crate::core::contracts::{
    installation::{Installation, Installed, RecordedInstance, Uninstalled},
    reply::{Failure, Meta, Reply, Success},
};
use crate::core::{domain::Scope, error::AppError};
use serde::Serialize;

/// Closed payload dispatch for migrated commands. Never add a Value fallback here.
#[derive(Serialize)]
#[serde(untagged)]
pub enum PublicData {
    Commands(Vec<crate::core::contracts::discovery::CommandSummary>),
    Description(Box<crate::core::contracts::discovery::Description>),
    Skills(Vec<crate::core::contracts::discovery::SkillSummary>),
    SkillGuide(crate::core::contracts::discovery::SkillGuide),
    SkillExport(crate::core::contracts::discovery::SkillExport),
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
    Typed(Reply<PublicData>),
    Scoped(crate::core::contracts::reply::ScopedReply<PublicData>),
    Collection(
        crate::core::contracts::reply::Paged<
            PublicData,
            crate::core::contracts::resource::CollectionPage,
        >,
    ),
}
#[derive(Serialize)]
pub struct Envelope {
    #[serde(flatten)]
    wire: Wire,
    #[serde(skip)]
    exit_code: u8,
}
impl Envelope {
    pub fn collection(
        data: Vec<crate::core::contracts::resource::Row>,
        scope: Scope,
        page_info: crate::core::contracts::resource::CollectionPage,
    ) -> Self {
        let reply = Success::scoped(PublicData::Resources(data), scope);
        Self {
            wire: Wire::Collection(crate::core::contracts::reply::Paged { reply, page_info }),
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
            let reply = Success::scoped(data.into(), scope);
            PageReply::Success(Paged { reply, page_info })
        };
        Self {
            wire: Wire::History(wire),
            exit_code: if partial { 7 } else { 0 },
        }
    }
    pub fn typed(data: impl Into<PublicData>) -> Self {
        Self {
            wire: Wire::Typed(Reply::Success(Success::new(data.into()))),
            exit_code: 0,
        }
    }
    pub fn failure(error: AppError) -> Self {
        let exit_code = error.exit_code;
        Self {
            wire: Wire::Typed(Reply::Failure(Failure::new(error))),
            exit_code,
        }
    }
    pub fn scoped(mut self, scope: Scope) -> Self {
        self.wire = match self.wire {
            Wire::Typed(Reply::Success(w)) => {
                Wire::Scoped(Reply::Success(Success::scoped(w.data, scope)))
            }
            Wire::Typed(Reply::Failure(mut w)) => {
                w.meta = Some(Meta { scope });
                Wire::Typed(Reply::Failure(w))
            }
            Wire::Scoped(Reply::Success(w)) => {
                Wire::Scoped(Reply::Success(Success::scoped(w.data, scope)))
            }
            Wire::Scoped(Reply::Failure(mut w)) => {
                w.meta = Some(Meta { scope });
                Wire::Scoped(Reply::Failure(w))
            }
            Wire::Collection(mut w) => {
                w.reply.metadata.meta = Meta { scope };
                Wire::Collection(w)
            }
            Wire::History(mut w) => {
                match &mut w {
                    crate::core::contracts::reply::PageReply::Success(w) => {
                        w.reply.metadata.meta = Meta { scope }
                    }
                    crate::core::contracts::reply::PageReply::Partial(w) => w.meta = Meta { scope },
                    crate::core::contracts::reply::PageReply::Failure(w) => {
                        w.meta = Some(Meta { scope })
                    }
                };
                Wire::History(w)
            }
        };
        self
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

impl From<Vec<crate::core::contracts::discovery::CommandSummary>> for PublicData {
    fn from(value: Vec<crate::core::contracts::discovery::CommandSummary>) -> Self {
        Self::Commands(value)
    }
}

impl From<crate::core::contracts::discovery::Description> for PublicData {
    fn from(value: crate::core::contracts::discovery::Description) -> Self {
        Self::Description(Box::new(value))
    }
}

impl From<Vec<crate::core::contracts::discovery::SkillSummary>> for PublicData {
    fn from(value: Vec<crate::core::contracts::discovery::SkillSummary>) -> Self {
        Self::Skills(value)
    }
}

impl From<crate::core::contracts::discovery::SkillGuide> for PublicData {
    fn from(value: crate::core::contracts::discovery::SkillGuide) -> Self {
        Self::SkillGuide(value)
    }
}

impl From<crate::core::contracts::discovery::SkillExport> for PublicData {
    fn from(value: crate::core::contracts::discovery::SkillExport) -> Self {
        Self::SkillExport(value)
    }
}

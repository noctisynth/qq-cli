//! Closed response dispatch over actual serialized contracts.
use crate::core::contracts::{
    installation::{Installation, Installed, RecordedInstance, Uninstalled},
    reply::{Failure, Meta, Reply, Success},
};
use crate::core::{domain::Scope, error::AppError};
use serde::Serialize;

// One dispatch declaration also generates conversions, avoiding parallel variant lists.
macro_rules! public_data {
 ($($variant:ident($ty:ty)),+ $(,)?)=>{
  #[derive(Serialize)] #[serde(untagged)] pub enum PublicData {$($variant($ty)),+}
  $(impl From<$ty> for PublicData {fn from(value:$ty)->Self {Self::$variant(value)}})+
 };
}
public_data! {

    Commands(Vec<crate::core::contracts::discovery::CommandSummary>),
    ApiCatalog(crate::core::contracts::api::Catalog),
    ApiDescription(crate::core::contracts::api::Description),
    ApiDraft(crate::core::contracts::api::Draft),
    ApiRequest(crate::core::contracts::api::Recorded),
    AccountReceipt(crate::core::contracts::account::Receipt),
    GroupReceipt(crate::core::contracts::group::Receipt),
    GroupDraft(crate::core::contracts::group::Draft),
    GroupMemberDetails(crate::core::contracts::group_query::MemberDetails),
    GroupDetails(crate::core::contracts::group_query::Details),
    GroupExtended(crate::core::contracts::group_query::Extended),
    GroupShare(crate::core::contracts::group_query::Share),
    GroupHonors(crate::core::contracts::group_query::Honors),
    GroupMuted(crate::core::contracts::group_query::Muted),
    GroupQuota(crate::core::contracts::group_query::Quota),
    GroupRequestBatch(crate::core::contracts::group_requests::Batch),
    GroupRequestReceipt(crate::core::contracts::group_requests::Receipt),
    GroupRequestDraft(crate::core::contracts::group_requests::Draft),
    ContactReceipt(crate::core::contracts::contact::Receipt),
    ContactDraft(crate::core::contracts::contact::Draft),
    ContactProfile(crate::core::contracts::contact::Profile),
    ContactCategories(crate::core::contracts::contact::Categories),
    ContactUnilateral(crate::core::contracts::contact::Unilateral),
    ContactRequests(crate::core::contracts::contact::Requests),
    ContactStatus(crate::core::contracts::contact::UserStatus),
    ContactLikes(crate::core::contracts::contact::Likes),
    AccountDraft(crate::core::contracts::account::Draft),
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
#[derive(Serialize)]
#[serde(untagged)]
enum Wire {
    Api(crate::core::contracts::api::CallReply),
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
    /// Typed adaptation for domain tasks sharing the explicit API execution engine.
    /// No serialization-to-JSON response bridge is involved.
    pub(crate) fn into_api_result(
        self,
    ) -> crate::core::error::Result<(crate::core::contracts::api::Call, Scope)> {
        use crate::core::contracts::api::CallReply;
        match self.wire {
            Wire::Api(CallReply::Success(w)) => Ok((w.data, w.metadata.meta.scope)),
            Wire::Api(CallReply::Failed(mut w)) => {
                if w.error.request_id.is_none() {
                    w.error.request_id = w.data.request_id;
                }
                if w.error.outcome.is_none() {
                    w.error.outcome = w.data.outcome;
                }
                Err(w.error.with_scope(w.meta.scope))
            }
            Wire::Api(CallReply::Error(w)) => Err(w.error),
            _ => Err(AppError::protocol()),
        }
    }
    pub fn api(
        data: crate::core::contracts::api::Call,
        scope: Scope,
        error: Option<AppError>,
    ) -> Self {
        use crate::core::contracts::{
            api::{CallReply, Failed},
            scalar::{No, WireVersion},
        };
        let exit_code = error.as_ref().map_or(0, |error| error.exit_code);
        let reply = match error {
            Some(error) => CallReply::Failed(Failed {
                schema_version: WireVersion,
                ok: No,
                data,
                error,
                meta: Meta { scope },
            }),
            None => CallReply::Success(Success::scoped(data, scope)),
        };
        Self {
            wire: Wire::Api(reply),
            exit_code,
        }
    }
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
            Wire::Api(mut reply) => {
                use crate::core::contracts::api::CallReply;
                match &mut reply {
                    CallReply::Success(w) => w.metadata.meta = Meta { scope },
                    CallReply::Failed(w) => w.meta = Meta { scope },
                    CallReply::Error(w) => w.meta = Some(Meta { scope }),
                }
                Wire::Api(reply)
            }
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

impl From<crate::core::contracts::discovery::Description> for PublicData {
    fn from(value: crate::core::contracts::discovery::Description) -> Self {
        Self::Description(Box::new(value))
    }
}

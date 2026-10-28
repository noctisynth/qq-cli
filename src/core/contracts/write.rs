//! Write receipts, dry runs, request observations and attachment results.
use super::scalar::{LocalId, No, Yes};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Image,
    File,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fingerprint {
    pub kind: Kind,
    #[schemars(length(min = 1, max = 512))]
    pub name: String,
    #[schemars(range(min = 1, max = 134217728_u64))]
    pub size_bytes: u64,
    #[schemars(regex(pattern = "^[a-f0-9]{64}$"))]
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pending,
    Accepted,
    Rejected,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StartOutcome {
    Pending,
    Dispatched,
    Unknown,
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub enum Transfer {
    #[serde(rename = "not_observable")]
    NotObservable,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptIdentity {
    #[schemars(regex(pattern = "^-?[1-9][0-9]*$"), length(max = 20))]
    pub message_id: String,
    #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
    pub conversation: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttachmentReceipt {
    #[serde(flatten)]
    pub identity: ReceiptIdentity,
    #[schemars(length(min = 1, max = 16))]
    pub attachments: Vec<Fingerprint>,
    pub file_transfer: Transfer,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Receipt {
    Attachments(AttachmentReceipt),
    Text(ReceiptIdentity),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Accepted {
    pub request_id: LocalId,
    pub outcome: AcceptedOutcome,
    pub cached: bool,
    pub receipt: Receipt,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AcceptedOutcome {
    Accepted,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Send,
    Reply,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DryRun {
    pub dry_run: Yes,
    pub operation: Operation,
    #[schemars(regex(pattern = "^(private|group):[1-9][0-9]{0,31}$"))]
    pub conversation: String,
    #[schemars(range(min = 1, max = 81))]
    pub segment_count: usize,
    pub reply: bool,
    #[schemars(length(max = 16))]
    pub attachments: Vec<Fingerprint>,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum WriteResult {
    Accepted(Accepted),
    DryRun(DryRun),
}
#[derive(Serialize, Deserialize, JsonSchema)]
pub enum UnknownTransfer {
    #[serde(rename = "unknown")]
    Unknown,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Uncertain {
    #[schemars(length(min = 1, max = 16))]
    pub attachments: Vec<Fingerprint>,
    pub file_transfer: UnknownTransfer,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rejected {
    pub code: String,
    pub attempted: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RecordedReceipt {
    Accepted(Receipt),
    Uncertain(Uncertain),
    Rejected(Rejected),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageRequest {
    pub request_id: LocalId,
    pub operation: String,
    pub outcome: Outcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "RecordedReceipt")]
    pub receipt: Option<RecordedReceipt>,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRequest {
    pub request_id: LocalId,
    pub operation: StartOperation,
    pub outcome: StartOutcome,
    pub instance_id: LocalId,
    pub run_id: LocalId,
    pub instance_state: String,
    pub identity_verified: No,
    pub next_action: super::action::NextAction,
}
#[derive(Serialize, JsonSchema)]
pub enum StartOperation {
    #[serde(rename = "napcat.start")]
    Start,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Request {
    Message(MessageRequest),
    Start(StartRequest),
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Download {
    #[schemars(regex(pattern = "^f1\\.[A-Za-z0-9_-]{22}\\.[A-Za-z0-9_-]{43}$"))]
    pub media_ref: String,
    pub kind: Kind,
    #[schemars(range(max = 134217728_u64))]
    pub size_bytes: u64,
    #[schemars(regex(pattern = "^[a-f0-9]{64}$"))]
    pub sha256: String,
}
#[derive(Serialize, JsonSchema)]
pub enum Cache {
    #[serde(rename = "attachments")]
    Attachments,
}
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pruned {
    pub cache: Cache,
    pub removed_files: u64,
    pub removed_bytes: u64,
    pub removed_entries: u64,
    pub preserved_entries: u64,
    pub retention_seconds: u64,
}

#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum RequestReply {
    Api(super::reply::ScopedReply<super::api::Recorded>),
    Message(super::reply::ScopedReply<MessageRequest>),
    Start(super::reply::Reply<StartRequest>),
}

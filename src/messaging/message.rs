use crate::core::contracts::{
    message::{
        Availability, KnownSegment, Media, Mention, Message as PublicMessage, Node, Segment,
        Sender, UnsupportedKind,
    },
    scalar::No,
};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::core::domain::{Conversation, Scope, decimal_id, identifier};
use crate::core::error::{AppError, Result};
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::transport::client::{Action, Backend};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Locator {
    pub scope: Scope,
    pub run_id: String,
    pub conversation: Conversation,
    pub backend_id: String,
    pub time_seconds: Option<i64>,
    pub sequence: Option<String>,
    pub sender_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct MediaLocator {
    pub message: Locator,
    pub kind: String,
    pub file_id: String,
    pub name: Option<String>,
    pub size: Option<u64>,
    #[serde(default)]
    pub opaque_id: bool,
    #[serde(default)]
    pub segment_index: Option<usize>,
    #[serde(default)]
    pub source_fingerprint: Option<String>,
}

#[derive(Debug)]
pub struct Message {
    pub locator: Locator,
    pub value: PublicMessage,
    pub text: String,
}

pub fn unsigned(value: &Value) -> Option<u64> {
    identifier(value).and_then(|s| s.parse().ok())
}

pub fn normalize(
    raw: &Value,
    scope: &Scope,
    run_id: &str,
    conversation: &Conversation,
    references: &References,
) -> Result<Message> {
    let backend_id = raw
        .get("message_id")
        .and_then(identifier)
        .ok_or_else(AppError::protocol)?;
    validate_peer(raw, scope, conversation)?;
    let sender_id = raw
        .get("user_id")
        .or_else(|| raw.get("sender").and_then(|s| s.get("user_id")))
        .and_then(identifier)
        .and_then(|s| decimal_id(&s).ok())
        .ok_or_else(AppError::protocol)?;
    if raw
        .get("sender")
        .and_then(|s| s.get("user_id"))
        .and_then(identifier)
        .is_some_and(|s| s != sender_id)
    {
        return Err(AppError::protocol());
    }
    let time = raw
        .get("time")
        .and_then(unsigned)
        .filter(|seconds| *seconds <= 253_402_300_799)
        .ok_or_else(AppError::protocol)? as i64;
    let sequence = match raw.get("real_seq") {
        None | Some(Value::Null) => None,
        Some(value) => Some(unsigned(value).ok_or_else(AppError::protocol)?.to_string()),
    };
    let locator = Locator {
        scope: scope.clone(),
        run_id: run_id.into(),
        conversation: conversation.clone(),
        backend_id: backend_id.clone(),
        time_seconds: Some(time),
        sequence,
        sender_id: Some(sender_id.clone()),
    };
    let message_ref = references.issue("m1", &locator)?;
    let mut budget = SegmentBudget { nodes: 0, bytes: 0 };
    let segments = normalize_segments(
        raw.get("message").ok_or_else(AppError::protocol)?,
        Some(&locator),
        references,
        false,
        0,
        &mut budget,
    )?;
    let text = searchable_text(&segments);
    let sender = Sender {
        user_id: sender_id,
        nickname: raw
            .get("sender")
            .and_then(|s| s.get("nickname"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    };
    Ok(Message {
        locator,
        text,
        value: PublicMessage {
            message_ref,
            id: backend_id,
            conversation: conversation.to_string(),
            sender,
            time_seconds: time,
            segments,
        },
    })
}

fn validate_peer(raw: &Value, scope: &Scope, conversation: &Conversation) -> Result<()> {
    if let Some(self_id) = raw.get("self_id")
        && identifier(self_id).as_deref() != Some(&scope.account)
    {
        return Err(AppError::protocol());
    }
    match conversation {
        Conversation::Group(id) => {
            if raw.get("message_type").and_then(Value::as_str) != Some("group")
                || raw.get("group_id").and_then(identifier).as_deref() != Some(id)
            {
                return Err(AppError::protocol());
            }
        }
        Conversation::Private(id) => {
            if raw.get("message_type").and_then(Value::as_str) != Some("private") {
                return Err(AppError::protocol());
            }
            let sender = raw
                .get("user_id")
                .and_then(identifier)
                .ok_or_else(AppError::protocol)?;
            if sender != *id && sender != scope.account {
                return Err(AppError::protocol());
            }
            if let Some(target) = raw.get("target_id") {
                let target = identifier(target).ok_or_else(AppError::protocol)?;
                if target != *id && target != scope.account {
                    return Err(AppError::protocol());
                }
            }
        }
    }
    Ok(())
}

pub fn load_locator(references: &References, token: &str) -> Result<Locator> {
    references.get("m1", token)
}

pub fn validate_locator<B: Backend>(locator: &Locator, session: &Session<'_, B>) -> Result<()> {
    if locator.scope.instance_id != session.instance.id || locator.run_id != session.instance.run_id
    {
        return Err(AppError::new(
            "STALE_REFERENCE",
            "Message reference belongs to another instance or backend run; locate it again in history.",
            4,
        ));
    }
    Ok(())
}

pub fn fetch<B: Backend>(
    locator: &Locator,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<Message> {
    fetch_raw(locator, session, references, deadline).map(|(_, message)| message)
}

pub(crate) fn fetch_raw<B: Backend>(
    locator: &Locator,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<(Value, Message)> {
    validate_locator(locator, session)?;
    let (raw,_) = session.read_in_scope(Some(&locator.conversation),Some(&locator.scope),deadline,|backend| {
        backend.call(Action::Message,&json!({"message_id":locator.backend_id}),deadline)
    }).map_err(|error| if error.code=="BACKEND_REJECTED" {
        AppError::new("STALE_REFERENCE","Backend could not resolve this message; this does not prove it was deleted or never sent.",4)
    } else {error})?;
    let message = normalize(
        &raw,
        &locator.scope,
        &locator.run_id,
        &locator.conversation,
        references,
    )?;
    if message.locator.backend_id != locator.backend_id
        || locator
            .time_seconds
            .is_some_and(|time| message.locator.time_seconds != Some(time))
        || locator
            .sequence
            .as_ref()
            .is_some_and(|seq| message.locator.sequence.as_ref() != Some(seq))
        || locator
            .sender_id
            .as_ref()
            .is_some_and(|id| message.locator.sender_id.as_ref() != Some(id))
    {
        return Err(AppError::new(
            "STALE_REFERENCE",
            "Backend message identity no longer matches the stored locator.",
            4,
        ));
    }
    Ok((raw, message))
}

pub fn get<B: Backend>(
    token: &str,
    expand: bool,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<PublicMessage> {
    let locator = load_locator(references, token)?;
    let (raw, mut result) = fetch_raw(&locator, session, references, deadline)?;
    if expand {
        let mut budget = SegmentBudget { nodes: 0, bytes: 0 };
        let mut segments = normalize_segments(
            raw.get("message").ok_or_else(AppError::protocol)?,
            Some(&locator),
            references,
            true,
            0,
            &mut budget,
        )?;
        ForwardExpansion {
            session,
            references,
            locator: &locator,
            deadline,
            active: std::collections::BTreeSet::new(),
            requests: 0,
            response_bytes: serde_json::to_vec(&raw)
                .map_err(|_| AppError::protocol())?
                .len(),
        }
        .resolve(&mut segments, 0, &mut budget)?;
        result.value.segments = segments;
    }
    Ok(result.value)
}

struct SegmentBudget {
    nodes: usize,
    bytes: usize,
}

struct ForwardExpansion<'a, 's, B: Backend> {
    session: &'a Session<'s, B>,
    references: &'a References,
    locator: &'a Locator,
    deadline: Instant,
    active: std::collections::BTreeSet<String>,
    requests: usize,
    response_bytes: usize,
}

impl<B: Backend> ForwardExpansion<'_, '_, B> {
    fn resolve(
        &mut self,
        segments: &mut [Segment],
        depth: usize,
        budget: &mut SegmentBudget,
    ) -> Result<()> {
        if depth > 4 || self.response_bytes > 4 * 1024 * 1024 {
            return Err(expansion_limit());
        }
        for segment in segments {
            if Instant::now() >= self.deadline {
                return Err(AppError::new(
                    "DEADLINE_EXCEEDED",
                    "Forward expansion exceeded the operation deadline.",
                    3,
                ));
            }
            let Segment::Known(KnownSegment::Forward {
                nodes,
                expanded,
                resource_id,
            }) = segment
            else {
                continue;
            };
            let resource = resource_id.take();
            if let Some(resource) = &resource {
                if depth >= 4 || self.requests >= 100 || !self.active.insert(resource.clone()) {
                    return Err(expansion_limit());
                }
                self.requests += 1;
                let (raw, _) = self.session.read_in_scope(
                    Some(&self.locator.conversation),
                    Some(&self.locator.scope),
                    self.deadline,
                    |backend| backend.call(Action::Forward, &json!({"id":resource}), self.deadline),
                )?;
                self.response_bytes = self.response_bytes.saturating_add(
                    serde_json::to_vec(&raw)
                        .map_err(|_| AppError::protocol())?
                        .len(),
                );
                if self.response_bytes > 4 * 1024 * 1024 {
                    return Err(expansion_limit());
                }
                *nodes = Some(normalize_nodes(
                    raw.get("messages").ok_or_else(AppError::protocol)?,
                    self.references,
                    depth + 1,
                    budget,
                )?);
                *expanded = true;
            }
            if let Some(nodes) = nodes {
                for node in nodes {
                    self.resolve(&mut node.segments, depth + 1, budget)?;
                }
            }
            if let Some(resource) = resource {
                self.active.remove(&resource);
            }
        }
        Ok(())
    }
}

fn normalize_segments(
    value: &Value,
    locator: Option<&Locator>,
    references: &References,
    expand: bool,
    depth: usize,
    budget: &mut SegmentBudget,
) -> Result<Vec<Segment>> {
    if depth > 4 {
        return Err(expansion_limit());
    }
    let raw = value.as_array().ok_or_else(AppError::protocol)?;
    let mut output = Vec::new();
    for (segment_index, segment) in raw.iter().enumerate() {
        budget.nodes += 1;
        if budget.nodes > 1000 {
            return Err(expansion_limit());
        }
        let kind = segment
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(AppError::protocol)?;
        let data = segment
            .get("data")
            .and_then(Value::as_object)
            .ok_or_else(AppError::protocol)?;
        let normalized = match kind {
            "text" => {
                let text = data
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(AppError::protocol)?;
                budget.bytes = budget.bytes.saturating_add(text.len());
                if budget.bytes > 4 * 1024 * 1024 {
                    return Err(expansion_limit());
                }
                Segment::Known(KnownSegment::Text { text: text.into() })
            }
            "at" => {
                let user = data
                    .get("qq")
                    .and_then(identifier)
                    .and_then(|id| decimal_id(&id).ok());
                match user {
                    Some(user) => Segment::Known(KnownSegment::Mention {
                        target: Mention::User { user_id: user },
                    }),
                    None => Segment::Known(KnownSegment::Mention {
                        target: Mention::Unsupported { supported: No },
                    }),
                }
            }
            "reply" => {
                let id = data
                    .get("id")
                    .and_then(identifier)
                    .ok_or_else(AppError::protocol)?;
                let mut message_ref = None;
                if let Some(locator) = locator {
                    let mut reply = locator.clone();
                    reply.backend_id = id;
                    reply.time_seconds = None;
                    reply.sequence = None;
                    reply.sender_id = None;
                    message_ref = Some(references.issue("m1", &reply)?);
                }
                Segment::Known(KnownSegment::Reply { message_ref })
            }
            "image" | "file" => media_segment(kind, data, locator, references, segment_index)?,
            "forward" => {
                let mut nodes = None;
                let mut expanded = false;
                let mut resource_id = None;
                if expand {
                    if let Some(content) = data.get("content") {
                        nodes = Some(normalize_nodes(content, references, depth + 1, budget)?);
                        expanded = true;
                    } else if let Some(id) = data.get("id").and_then(Value::as_str) {
                        if id.is_empty() || id.len() > 1024 || id.chars().any(char::is_control) {
                            return Err(AppError::protocol());
                        }
                        resource_id = Some(id.into());
                    }
                }
                Segment::Known(KnownSegment::Forward {
                    expanded,
                    nodes,
                    resource_id,
                })
            }
            _ => Segment::Unsupported {
                kind: UnsupportedKind::new(kind),
                supported: No,
            },
        };
        output.push(normalized);
    }
    Ok(output)
}

fn normalize_nodes(
    value: &Value,
    references: &References,
    depth: usize,
    budget: &mut SegmentBudget,
) -> Result<Vec<Node>> {
    if depth > 4 {
        return Err(expansion_limit());
    }
    let nodes = value.as_array().ok_or_else(AppError::protocol)?;
    if nodes.len() > 100 {
        return Err(expansion_limit());
    }
    nodes
        .iter()
        .map(|node| {
            budget.nodes += 1;
            if budget.nodes > 1000 {
                return Err(expansion_limit());
            }
            let data = if node.get("type").and_then(Value::as_str) == Some("node") {
                node.get("data").ok_or_else(AppError::protocol)?
            } else {
                node
            };
            let content = data
                .get("content")
                .or_else(|| data.get("message"))
                .ok_or_else(AppError::protocol)?;
            let segments = normalize_segments(content, None, references, true, depth, budget)?;
            let sender_id = data.get("user_id").and_then(identifier);
            let nickname = data
                .get("nickname")
                .or_else(|| data.get("sender").and_then(|s| s.get("nickname")))
                .and_then(Value::as_str)
                .map(str::to_owned);
            Ok(Node {
                segments,
                sender_id,
                nickname,
            })
        })
        .collect()
}

fn media_segment(
    kind: &str,
    data: &Map<String, Value>,
    locator: Option<&Locator>,
    references: &References,
    segment_index: usize,
) -> Result<Segment> {
    let mut result = Media {
        availability: Availability::Unavailable {
            downloadable: No,
            media_ref: None,
        },
        name: None,
        size_bytes: None,
    };
    let file = data
        .get("file_id")
        .or_else(|| data.get("file"))
        .and_then(Value::as_str);
    let name = data
        .get("file")
        .and_then(Value::as_str)
        .filter(|name| !name.contains("://") && !name.contains(['?', '#']))
        .and_then(|s| {
            let name = s.rsplit(['/', '\\']).next().unwrap_or_default();
            crate::napcat::install::archive::member_path(name)
                .ok()
                .map(|_| name.to_owned())
        });
    if let Some(name) = &name {
        result.name = Some(name.clone());
    }
    let size = data.get("file_size").and_then(unsigned);
    if let Some(size) = size {
        result.size_bytes = Some(size);
    }
    if let (Some(file_id), Some(locator)) = (file, locator)
        && !file_id.is_empty()
        && file_id.len() <= 1024
        && ((!file_id.contains(['/', '\\', ':', '?', '#'])
            && !file_id.chars().any(char::is_control)
            && crate::napcat::install::archive::member_path(file_id).is_ok())
            || (kind == "file"
                && matches!(locator.conversation, Conversation::Group(_))
                && data.get("file_id").and_then(Value::as_str) == Some(file_id)
                && is_group_file_uuid(file_id)))
    {
        let media_ref = references.issue(
            "f1",
            &MediaLocator {
                message: locator.clone(),
                kind: kind.into(),
                file_id: file_id.into(),
                name,
                size,
                segment_index: Some(segment_index),
                source_fingerprint: if kind == "image" {
                    data.get("url")
                        .and_then(Value::as_str)
                        .and_then(crate::messaging::media::source::image_identity)
                } else {
                    None
                },
                opaque_id: kind == "file"
                    && data
                        .get("file_id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| id == file_id),
            },
        )?;
        let downloadable = (kind == "image"
            && crate::messaging::media::source::can_resolve_image(
                data.get("url").and_then(Value::as_str),
            ))
            || (kind == "file"
                && data
                    .get("file_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == file_id));
        result.reference(media_ref, downloadable);
    }
    Ok(Segment::Known(if kind == "image" {
        KnownSegment::Image { media: result }
    } else {
        KnownSegment::File { media: result }
    }))
}

// QQ group fileUuid is an opaque API identifier, sometimes prefixed with '/'.
// Accept this exact wire shape without interpreting it as a filesystem path.
fn is_group_file_uuid(value: &str) -> bool {
    let Some(uuid) = value.strip_prefix('/') else {
        return false;
    };
    uuid.len() == 36
        && uuid.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn searchable_text(segments: &[Segment]) -> String {
    segments
        .iter()
        .filter_map(|s| match s {
            Segment::Known(KnownSegment::Text { text }) => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

fn expansion_limit() -> AppError {
    AppError::new(
        "EXPANSION_LIMIT",
        "Forwarded message exceeds bounded expansion limits.",
        4,
    )
}

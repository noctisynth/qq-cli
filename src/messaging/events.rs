//! Bounded event normalization, independent of the WebSocket transport.
use std::collections::HashSet;
use std::time::Instant;
mod checkpoint;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::core::domain::{Conversation, Scope, decimal_id, identifier};
use crate::core::error::{AppError, Result};
use crate::messaging::message;
use crate::storage::config::Config;
use crate::storage::references::References;

pub struct Watch<'a> {
    scope: Scope,
    run: String,
    target: Option<Conversation>,
    config: &'a Config,
    references: &'a References,
    deadline: Instant,
    limit: usize,
    seen: HashSet<String>,
    frames: usize,
    emitted: usize,
    duplicates: usize,
    filtered: usize,
    heartbeats: usize,
    gaps: usize,
    bytes: usize,
    failed: bool,
    checkpoint: checkpoint::Checkpoint,
    recovery: Option<Value>,
}

impl<'a> Watch<'a> {
    pub fn new(
        scope: Scope,
        run: String,
        target: Option<Conversation>,
        config: &'a Config,
        references: &'a References,
        limit: u16,
        deadline: Instant,
    ) -> Result<Self> {
        if !config.permissions.events || !config.permissions.read {
            return Err(AppError::new(
                "PERMISSION_DENIED",
                "The local policy does not permit event observation.",
                4,
            ));
        }
        if !(1..=1000).contains(&limit) {
            return Err(AppError::invalid());
        }
        decimal_id(&scope.account).map_err(|_| AppError::invalid())?;
        crate::core::domain::request_key(&scope.instance_id).map_err(|_| AppError::invalid())?;
        crate::core::domain::request_key(&run).map_err(|_| AppError::invalid())?;
        if target
            .as_ref()
            .is_some_and(|target| !allowed(config, target))
        {
            return Err(AppError::new(
                "TARGET_DENIED",
                "The selected conversation is outside the local policy.",
                4,
            ));
        }
        Ok(Self {
            checkpoint: checkpoint::Checkpoint::new(scope.clone(), run.clone(), target.clone()),
            recovery: None,
            scope,
            run,
            target,
            config,
            references,
            deadline,
            limit: usize::from(limit),
            seen: HashSet::new(),
            frames: 0,
            emitted: 0,
            duplicates: 0,
            filtered: 0,
            heartbeats: 0,
            gaps: 0,
            bytes: 0,
            failed: false,
        })
    }

    pub fn ready(&self, self_messages: Option<bool>) -> Value {
        json!({"type":"ready","schemaVersion":1,"scope":self.scope,"selfMessages":self_messages,"coverage":"best_effort"})
    }

    pub fn full(&self) -> bool {
        self.emitted >= self.limit
    }

    pub fn ingest(&mut self, frame: &[u8]) -> Result<Option<Value>> {
        if self.failed {
            return Err(AppError::new(
                "EVENT_STREAM_STOPPED",
                "Event observation stopped after an earlier error.",
                4,
            ));
        }
        let result = self.ingest_frame(frame);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn ingest_frame(&mut self, frame: &[u8]) -> Result<Option<Value>> {
        if Instant::now() >= self.deadline {
            return Err(AppError::new(
                "DEADLINE",
                "Event observation reached its deadline.",
                3,
            ));
        }
        if self.full() || self.frames >= 10_000 || frame.len() > 4 * 1024 * 1024 {
            return Err(limit_error());
        }
        self.frames += 1;
        let raw: Value = serde_json::from_slice(frame).map_err(|_| AppError::protocol())?;
        if raw.get("self_id").and_then(identifier).as_deref() != Some(&self.scope.account) {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Event identity does not match the selected account; observation must stop.",
                4,
            ));
        }
        let post = raw
            .get("post_type")
            .and_then(Value::as_str)
            .ok_or_else(AppError::protocol)?;
        if post == "meta_event" {
            if raw.get("meta_event_type").and_then(Value::as_str) == Some("heartbeat") {
                self.heartbeats += 1;
            }
            return Ok(None);
        }
        let target = conversation(&raw, &self.scope, post)?;
        if target.as_ref().is_some_and(|target| {
            !allowed(self.config, target)
                || self
                    .target
                    .as_ref()
                    .is_some_and(|expected| expected != target)
        }) || target.is_none()
            && (self.target.is_some() || !self.config.permissions.targets.is_empty())
        {
            self.filtered += 1;
            return Ok(None);
        }
        let identity = json!({"scope":self.scope,"run":self.run,"post":if post=="message_sent" {"message"} else {post},"conversation":target,
            "id":raw.get("message_id").and_then(identifier),"sequence":raw.get("real_seq").and_then(identifier),"time":raw.get("time").and_then(identifier),
            "sender":raw.get("user_id").and_then(identifier),"notice":raw.get("notice_type").and_then(Value::as_str),"subType":raw.get("sub_type").and_then(Value::as_str),
            "operator":raw.get("operator_id").and_then(identifier),"target":raw.get("target_id").and_then(identifier)});
        let event_id = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&identity).map_err(|_| AppError::protocol())?)
        );
        let event = if matches!(post, "message" | "message_sent") {
            let target = target.as_ref().ok_or_else(AppError::protocol)?;
            let message =
                message::normalize(&raw, &self.scope, &self.run, target, self.references)?;
            self.message_event(message.locator, message.value, "live")?
        } else {
            let mut event =
                json!({"type":"notice","eventId":event_id,"conversation":target,"supported":false});
            if let Some(notice) = normalize_notice(&raw, post)? {
                event["supported"] = json!(true);
                event["notice"] = notice;
            }
            event
        };
        self.emit(event)
    }

    fn emit(&mut self, event: Value) -> Result<Option<Value>> {
        let event_id = event["eventId"]
            .as_str()
            .ok_or_else(AppError::protocol)?
            .to_owned();
        if self.seen.contains(&event_id) || self.checkpoint.contains(&event_id) {
            self.duplicates += 1;
            return Ok(None);
        }
        if self.full() || Instant::now() >= self.deadline {
            return Err(limit_error());
        }
        let size = serde_json::to_vec(&event)
            .map_err(|_| AppError::protocol())?
            .len()
            + if self.target.is_some() { 100 } else { 0 };
        if self.bytes.saturating_add(size) > self.config.response_bytes {
            return Err(limit_error());
        }
        self.bytes += size;
        self.emitted += 1;
        self.seen.insert(event_id);
        if event["type"] == "message" {
            self.checkpoint.remember(
                event["eventId"]
                    .as_str()
                    .ok_or_else(AppError::protocol)?
                    .into(),
            );
        }
        Ok(Some(event))
    }

    fn message_event(
        &self,
        locator: message::Locator,
        value: Value,
        source: &str,
    ) -> Result<Value> {
        if locator.scope != self.scope
            || locator.run_id != self.run
            || self
                .target
                .as_ref()
                .is_some_and(|t| t != &locator.conversation)
        {
            return Err(AppError::new(
                "EVENT_CHECKPOINT_STALE",
                "Recovered message scope does not match this observation.",
                4,
            ));
        }
        Ok(
            json!({"type":"message","eventId":checkpoint::message_id(&locator)?,"source":source,"direction":if locator.sender_id.as_deref()==Some(&self.scope.account) {"outgoing"} else {"incoming"},"message":value}),
        )
    }
    pub(crate) fn recover(&mut self, value: Value) -> Result<Option<Value>> {
        let token = value["messageRef"]
            .as_str()
            .ok_or_else(AppError::protocol)?;
        let locator = self.references.get("m1", token)?;
        self.emit(self.message_event(locator, value, "history")?)
    }
    pub(crate) fn restore(&mut self, token: &str) -> Result<()> {
        self.checkpoint.restore(self.references, token)
    }
    pub(crate) fn since(&self) -> i64 {
        self.checkpoint.since
    }
    pub(crate) fn remaining(&self) -> u16 {
        (self.limit - self.emitted) as u16
    }
    pub(crate) fn attach_checkpoint(&self, value: &mut Value) -> Result<()> {
        if let Some(token) = self.checkpoint.token(self.references)? {
            value["checkpoint"] = json!(token);
        }
        Ok(())
    }

    pub fn gap(&mut self, reason: &'static str) -> Value {
        self.gaps += 1;
        json!({"type":"gap","reason":reason,"recovered":false,"coverage":"unknown"})
    }

    pub(crate) fn recovered_page(&mut self, info: &Value, before: u16) {
        let old = self.recovery.take().unwrap_or_else(|| json!({}));
        self.recovery = Some(json!({"attempts":old["attempts"].as_u64().unwrap_or(0)+1,
            "scanned":old["scanned"].as_u64().unwrap_or(0)+info["scanned"].as_u64().unwrap_or(0),
            "emitted":old["emitted"].as_u64().unwrap_or(0)+u64::from(before-self.remaining()),
            "sinceSeconds":self.since(),"stopReason":info["stopReason"],"historyComplete":info["complete"],"coverage":"backend_accessible"}));
    }
    pub fn summary(&self, reason: &'static str) -> Value {
        let mut value = json!({"type":"summary","reason":reason,"emitted":self.emitted,"duplicates":self.duplicates,
            "filtered":self.filtered,"heartbeats":self.heartbeats,"frames":self.frames,"gaps":self.gaps,
            "bytes":self.bytes,"coverage":"best_effort"});
        if let Some(recovery) = &self.recovery {
            value["recovery"] = recovery.clone();
        }
        value
    }
}

fn allowed(config: &Config, target: &Conversation) -> bool {
    config.permissions.targets.is_empty()
        || config.permissions.targets.contains(&target.to_string())
}

fn conversation(raw: &Value, scope: &Scope, post: &str) -> Result<Option<Conversation>> {
    let positive = |key| {
        raw.get(key)
            .and_then(identifier)
            .and_then(|id| decimal_id(&id).ok())
            .ok_or_else(AppError::protocol)
    };
    if matches!(post, "message" | "message_sent") {
        return match raw.get("message_type").and_then(Value::as_str) {
            Some("group") => Ok(Some(Conversation::Group(positive("group_id")?))),
            Some("private") => {
                let sender = positive("user_id")?;
                let peer = if sender == scope.account {
                    positive("target_id")?
                } else {
                    sender
                };
                Ok(Some(Conversation::Private(peer)))
            }
            _ => Err(AppError::protocol()),
        };
    }
    if post == "notice"
        && matches!(
            raw.get("notice_type").and_then(Value::as_str),
            Some("friend_recall" | "friend_add")
        )
    {
        if raw.get("group_id").is_some() {
            return Err(AppError::protocol());
        }
        return Ok(Some(Conversation::Private(positive("user_id")?)));
    }
    if raw.get("group_id").is_some() {
        return Ok(Some(Conversation::Group(positive("group_id")?)));
    }
    Ok(None)
}

fn normalize_notice(raw: &Value, post: &str) -> Result<Option<Value>> {
    if post != "notice" {
        return Ok(None);
    }
    let kind = raw
        .get("notice_type")
        .and_then(Value::as_str)
        .ok_or_else(AppError::protocol)?;
    if !matches!(
        kind,
        "group_recall"
            | "friend_recall"
            | "group_increase"
            | "group_decrease"
            | "group_admin"
            | "friend_add"
    ) {
        return Ok(None);
    }
    if kind.starts_with("group_") {
        raw.get("group_id")
            .and_then(identifier)
            .and_then(|id| decimal_id(&id).ok())
            .ok_or_else(AppError::protocol)?;
    }
    let user = raw
        .get("user_id")
        .and_then(identifier)
        .ok_or_else(AppError::protocol)?;
    let user = if user == "0" && kind.starts_with("group_") {
        Value::Null
    } else {
        json!(decimal_id(&user).map_err(|_| AppError::protocol())?)
    };
    let time = raw
        .get("time")
        .and_then(message::unsigned)
        .filter(|time| *time <= 253_402_300_799)
        .ok_or_else(AppError::protocol)?;
    let mut notice = json!({"kind":kind,"userId":user,"timeSeconds":time});
    if matches!(kind, "group_recall" | "friend_recall") {
        notice["messageId"] = json!(
            raw.get("message_id")
                .and_then(identifier)
                .ok_or_else(AppError::protocol)?
        );
    }
    if matches!(kind, "group_recall" | "group_increase" | "group_decrease") {
        let operator = raw
            .get("operator_id")
            .and_then(message::unsigned)
            .ok_or_else(AppError::protocol)?;
        notice["operatorId"] = if operator == 0 {
            Value::Null
        } else {
            json!(operator.to_string())
        };
    }
    if matches!(kind, "group_increase" | "group_decrease" | "group_admin") {
        let subtype = raw
            .get("sub_type")
            .and_then(Value::as_str)
            .ok_or_else(AppError::protocol)?;
        let recognized = match kind {
            "group_increase" => matches!(subtype, "approve" | "invite"),
            "group_decrease" => matches!(subtype, "leave" | "kick" | "kick_me"),
            _ => matches!(subtype, "set" | "unset"),
        };
        if !recognized {
            return Ok(None);
        }
        notice["subType"] = json!(subtype);
    }
    Ok(Some(notice))
}

fn limit_error() -> AppError {
    AppError::new(
        "EVENT_STREAM_LIMIT",
        "Event observation reached a frame, event or output budget.",
        4,
    )
}

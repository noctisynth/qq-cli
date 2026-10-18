//! One caller intent, one backend write attempt. Transport/business failures may follow side effects.
use std::time::Instant;

use serde_json::{Value, json};

use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::message::{self, Locator};
use crate::messaging::session::Session;
use crate::requests::ledger::{self, Begin, Outcome, Record};
use crate::storage::references::References;
use crate::storage::state::Store;
use crate::transport::client::{Action, Backend};

pub struct Plan {
    pub conversation: Conversation,
    pub segments: Vec<Value>,
    pub reply: Option<Locator>,
    pub attachments: Vec<Attachment>,
}

pub struct Attachment {
    pub kind: crate::messaging::attachments::Kind,
    pub path: std::path::PathBuf,
}

impl Plan {
    fn operation(&self) -> &'static str {
        if self.reply.is_some() {
            "reply"
        } else {
            "send"
        }
    }
    fn fingerprint(&self, attachments: &[Value]) -> Value {
        // Reply handles can be re-issued; nonce and instance/run are not message content.
        let reply = self.reply.as_ref().map(|locator| {
            json!({"id":locator.backend_id,"time":locator.time_seconds,
            "sequence":locator.sequence,"sender":locator.sender_id})
        });
        let mut value =
            json!({"conversation":self.conversation,"segments":self.segments,"reply":reply});
        if !attachments.is_empty() {
            value["attachments"] = json!(attachments);
        }
        value
    }
}

pub fn execute<B: Backend>(
    plan: Plan,
    key: Option<&str>,
    dry_run: bool,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    session.authorize(Some(&plan.conversation), true)?;
    if plan.segments.is_empty() && plan.attachments.is_empty() || plan.attachments.len() > 16 {
        return Err(AppError::invalid());
    }
    let mut snapshots = plan
        .attachments
        .iter()
        .map(|input| {
            crate::messaging::attachments::prepare(&input.path, input.kind, store, deadline)
        })
        .collect::<Result<Vec<_>>>()?;
    let attachment_fingerprints = snapshots
        .iter()
        .map(crate::messaging::attachments::Snapshot::fingerprint)
        .collect::<Vec<_>>();
    if let Some(reply) = &plan.reply
        && reply.conversation != plan.conversation
    {
        return Err(AppError::invalid());
    }
    if dry_run {
        if let Some(reply) = &plan.reply {
            message::validate_locator(reply, session)?;
        }
        let info = session.verify(deadline)?;
        validate_mentions(&plan, session, &info.account, deadline)?;
        if plan
            .reply
            .as_ref()
            .is_some_and(|reply| reply.scope.account != info.account)
        {
            return Err(account_changed());
        }
        return Ok(Envelope::success(
            json!({"dryRun":true,"operation":plan.operation(),"conversation":plan.conversation,
            "segmentCount":plan.segments.len()+snapshots.len(),"reply":plan.reply.is_some(),"attachments":attachment_fingerprints}),
        )
        .scoped(Scope {
            instance_id: session.instance.id.clone(),
            account: info.account,
        }));
    }
    let key = key.ok_or_else(AppError::invalid)?;
    // A known cached outcome is available without probing an offline backend.
    let account = match &session.instance.account {
        Some(account) => account.clone(),
        None => session.verify(deadline)?.account,
    };
    let scope = Scope {
        instance_id: session.instance.id.clone(),
        account,
    };
    if plan
        .reply
        .as_ref()
        .is_some_and(|reply| reply.scope.account != scope.account)
    {
        return Err(account_changed());
    }
    let intent = match ledger::begin(
        store,
        key,
        plan.operation(),
        &scope,
        &plan.fingerprint(&attachment_fingerprints),
        deadline,
    )? {
        Begin::Existing(record) => return result(record, true, &plan.conversation),
        Begin::Fresh(intent) => intent,
    };
    let mut retained = Vec::new();
    // The durable intent exists before any write. Re-check after waiting for the process lock.
    let preflight = (|| {
        let info = session.verify(deadline)?;
        if info.account != scope.account {
            return Err(account_changed());
        }
        if let Some(reply) = &plan.reply {
            message::fetch(
                reply,
                session,
                references.ok_or_else(AppError::storage)?,
                deadline,
            )?;
        }
        validate_mentions(&plan, session, &info.account, deadline)?;
        for snapshot in snapshots.drain(..) {
            retained.push(snapshot.retain_for_intent(
                &intent,
                references.ok_or_else(AppError::storage)?,
                deadline,
            )?);
        }
        if Instant::now() >= deadline {
            return Err(AppError::new(
                "DEADLINE_EXCEEDED",
                "Deadline expired before the write attempt.",
                3,
            ));
        }
        Ok(())
    })();
    if let Err(mut error) = preflight {
        intent.finish(
            Outcome::Rejected,
            Some(json!({"code":error.code,"attempted":false})),
        )?;
        error.outcome = Some("rejected");
        error.request_id = Some(key.into());
        return Err(error.with_scope(scope.clone()));
    }
    let (action, id_key, id) = match &plan.conversation {
        Conversation::Private(id) => (Action::SendPrivate, "user_id", id),
        Conversation::Group(id) => (Action::SendGroup, "group_id", id),
    };
    let mut segments = plan.segments.clone();
    for attachment in &retained {
        let fingerprint = attachment.fingerprint();
        let path = attachment.path().to_str().ok_or_else(AppError::storage)?;
        segments.push(
            json!({"type":fingerprint["kind"],"data":{"file":path,"name":fingerprint["name"]}}),
        );
    }
    let uncertain = if retained.is_empty() {
        None
    } else {
        Some(json!({"attachments":attachment_fingerprints,"fileTransfer":"unknown"}))
    };
    if let Some(reply) = &plan.reply {
        segments.insert(0, json!({"type":"reply","data":{"id":reply.backend_id}}));
    }
    let response = session.backend.call(
        action,
        &json!({id_key:id,"message":segments,"auto_escape":true}),
        deadline,
    );
    let receipt = match response {
        Ok(raw) => match raw
            .get("message_id")
            .and_then(crate::messaging::receipts::message_id)
        {
            Some(id) => {
                let mut receipt = json!({"messageId":id,"conversation":plan.conversation});
                if !retained.is_empty() {
                    receipt["attachments"] = json!(attachment_fingerprints);
                    receipt["fileTransfer"] = json!("not_observable");
                }
                receipt
            }
            None => {
                intent.finish(Outcome::Unknown, uncertain)?;
                return Err(ledger::unknown(key).with_scope(scope.clone()));
            }
        },
        Err(error) => {
            // A backend error can occur after sending. Only HTTP authentication refusal is known not to execute the action.
            if error.code == "BACKEND_AUTH" {
                intent.finish(
                    Outcome::Rejected,
                    Some(json!({"code":error.code,"attempted":true})),
                )?;
                let mut error = error;
                error.outcome = Some("rejected");
                error.request_id = Some(key.into());
                return Err(error.with_scope(scope.clone()));
            }
            intent.finish(Outcome::Unknown, uncertain)?;
            return Err(ledger::unknown(key).with_scope(scope.clone()));
        }
    };
    match session.verify(deadline) {
        Ok(after) if after.account == scope.account => (),
        _ => {
            intent.finish(Outcome::Unknown, Some(receipt))?;
            return Err(ledger::unknown(key).with_scope(scope.clone()));
        }
    }
    result(
        intent.finish(Outcome::Accepted, Some(receipt))?,
        false,
        &plan.conversation,
    )
}

/// Resolve explicit mentions once per attempt; cached receipts do not depend on current membership.
fn validate_mentions<B: Backend>(
    plan: &Plan,
    session: &Session<'_, B>,
    account: &str,
    deadline: Instant,
) -> Result<()> {
    let mut requested = std::collections::BTreeSet::new();
    for segment in &plan.segments {
        if segment.get("type").and_then(Value::as_str) == Some("at") {
            let id = segment
                .pointer("/data/qq")
                .and_then(Value::as_str)
                .ok_or_else(AppError::invalid)?;
            crate::core::domain::decimal_id(id).map_err(|_| AppError::invalid())?;
            requested.insert(id.to_owned());
        }
    }
    if requested.is_empty() {
        return Ok(());
    }
    let Conversation::Group(group) = &plan.conversation else {
        return Err(AppError::new(
            "MENTION_UNSUPPORTED",
            "Mentions require a group conversation.",
            2,
        ));
    };
    if requested.len() > 64 {
        return Err(AppError::invalid());
    }
    let raw = session.backend.call(
        Action::GroupMembers,
        &json!({"group_id":group,"no_cache":true}),
        deadline,
    )?;
    let members = raw.as_array().ok_or_else(AppError::protocol)?;
    for value in members {
        let member = crate::messaging::read::member(value)?;
        requested.remove(member["userId"].as_str().ok_or_else(AppError::protocol)?);
    }
    if !requested.is_empty() {
        return Err(AppError::new(
            "MENTION_NOT_MEMBER",
            "An explicitly mentioned account was not found in the current group membership response.",
            4,
        ));
    }
    // Membership reads can race an account change; do not write under the earlier identity.
    let after = session.verify(deadline)?;
    if after.account != account {
        return Err(account_changed());
    }
    Ok(())
}

fn result(record: Record, cached: bool, expected: &Conversation) -> Result<Envelope> {
    match record.outcome {
        Outcome::Accepted => {
            crate::messaging::receipts::validate(
                record.receipt.as_ref().unwrap_or(&Value::Null),
                Some(expected),
            )
            .map_err(|error| error.with_scope(record.scope.clone()))?;
            Ok(Envelope::success(
                json!({"requestId":record.request_id,"outcome":"accepted","cached":cached,
            "receipt":record.receipt}),
            )
            .scoped(record.scope))
        }
        Outcome::Rejected => {
            let mut error = AppError::new(
                "WRITE_REJECTED",
                "The recorded request was rejected; this key will not execute again.",
                4,
            );
            error.outcome = Some("rejected");
            error.request_id = Some(record.request_id);
            Err(error.with_scope(record.scope))
        }
        Outcome::Pending | Outcome::Unknown => {
            Err(ledger::unknown(&record.request_id).with_scope(record.scope))
        }
    }
}

fn account_changed() -> AppError {
    AppError::new(
        "ACCOUNT_CHANGED",
        "Write identity does not match the referenced or registered account; no write was attempted.",
        4,
    )
}

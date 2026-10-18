//! Offline request lookup. Durable dispatch is never evidence of an online account.
use crate::{
    credentials::Credentials,
    error::{AppError, Result},
    output::Envelope,
    references::References,
    state::Store,
};
use serde_json::json;
use std::time::Instant;

pub fn get(
    store: &Store,
    key: &str,
    instance: Option<&str>,
    credentials: &impl Credentials,
    deadline: Instant,
) -> Result<Envelope> {
    crate::deadline::check(deadline)?;
    let has_start = crate::request_keys::has_start(store, key)?;
    match crate::ledger::get(store, key) {
        Ok(record) => {
            if has_start {
                return Err(crate::request_keys::conflict());
            }
            if matches!(record.operation.as_str(), "send" | "reply")
                && record.outcome == crate::ledger::Outcome::Accepted
            {
                crate::receipts::validate(
                    record.receipt.as_ref().unwrap_or(&serde_json::Value::Null),
                    None,
                )
                .map_err(|error| error.with_scope(record.scope.clone()))?;
            }
            check_instance(instance, &record.scope.instance_id)?;
            let mut data = json!({"requestId":record.request_id,"operation":record.operation,"outcome":record.outcome});
            if let Some(receipt) = record.receipt {
                data["receipt"] = receipt;
            }
            Ok(Envelope::success(data).scoped(record.scope))
        }
        Err(error) if error.code == "REQUEST_NOT_FOUND" && has_start => {
            let refs = References::existing(store, credentials)?;
            start(store, &refs, key, instance, deadline)
        }
        Err(error) => Err(error),
    }
}
fn check_instance(explicit: Option<&str>, recorded: &str) -> Result<()> {
    if explicit.is_some_and(|id| id != recorded) {
        Err(AppError::new(
            "INSTANCE_CONFLICT",
            "Explicit instance conflicts with the recorded request.",
            2,
        ))
    } else {
        Ok(())
    }
}
fn start(
    store: &Store,
    refs: &References,
    key: &str,
    explicit: Option<&str>,
    deadline: Instant,
) -> Result<Envelope> {
    let creation = crate::instance_creation::checkpoint(store, refs, key)?;
    let dispatch = match crate::start_intent::get(store, refs, key) {
        Ok(record) => Some(record),
        Err(error) if error.code == "REQUEST_NOT_FOUND" => None,
        Err(error) => return Err(error),
    };
    if let (Some(creation), Some(dispatch)) = (&creation, &dispatch)
        && (creation.plan.instance_id != dispatch.target.instance_id
            || creation.plan.run_id != dispatch.target.run_id
            || creation.installation_id != dispatch.target.installation_id
            || creation.installation_digest != dispatch.target.installation_digest)
    {
        return Err(AppError::new(
            "REQUEST_RECORD_CONFLICT",
            "Signed creation and dispatch records disagree; no state was changed.",
            6,
        ));
    }
    let (instance_id, run_id, installation_id) = if let Some(record) = &creation {
        (
            &record.plan.instance_id,
            &record.plan.run_id,
            &record.installation_id,
        )
    } else if let Some(record) = &dispatch {
        (
            &record.target.instance_id,
            &record.target.run_id,
            &record.target.installation_id,
        )
    } else {
        return Err(AppError::storage());
    };
    check_instance(explicit, instance_id)?;
    let registry = store.read()?;
    let current = registry.instances.iter().find(|i| &i.id == instance_id);
    let state = match current {
        None => "unregistered",
        Some(i) if &i.run_id != run_id || &i.installation_id != installation_id => "changed",
        Some(i) => &i.state,
    };
    if !matches!(
        state,
        "unregistered"
            | "changed"
            | "preparing"
            | "prepared"
            | "starting"
            | "login_required"
            | "ready"
            | "stopping"
            | "stopped"
            | "offline"
            | "failed"
            | "unknown"
    ) {
        return Err(AppError::storage());
    }
    let outcome = dispatch
        .as_ref()
        .map(|r| &r.outcome)
        .unwrap_or(&crate::start_intent::Outcome::Pending);
    let next = if matches!(state, "unregistered" | "changed") {
        json!({"command":"napcat.instances","args":{}})
    } else {
        json!({"command":"status","args":{"instance":instance_id}})
    };
    crate::deadline::check(deadline)?;
    Ok(Envelope::success(
        json!({"requestId":key,"operation":"napcat.start","outcome":outcome,
        "instanceId":instance_id,"runId":run_id,"instanceState":state,"identityVerified":false,"nextAction":next}),
    ))
}

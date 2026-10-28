//! Offline request lookup. Durable dispatch is never evidence of an online account.
use crate::core::contracts::{
    action::NextAction,
    scalar::{LocalId, No},
    write::{MessageRequest, RecordedReceipt, StartOperation, StartRequest},
};
use crate::{
    core::error::{AppError, Result},
    core::output::Envelope,
    storage::credentials::Credentials,
    storage::references::References,
    storage::state::Store,
};
use std::time::Instant;

pub fn get(
    store: &Store,
    key: &str,
    instance: Option<&str>,
    credentials: &impl Credentials,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let has_start = crate::requests::keys::has_start(store, key)?;
    match crate::requests::ledger::get(store, key) {
        Ok(record) => {
            if has_start {
                return Err(crate::requests::keys::conflict());
            }
            if matches!(record.operation.as_str(), "send" | "reply")
                && record.outcome == crate::requests::ledger::Outcome::Accepted
            {
                crate::messaging::receipts::validate(
                    record.receipt.as_ref().unwrap_or(&serde_json::Value::Null),
                    None,
                )
                .map_err(|error| error.with_scope(record.scope.clone()))?;
            }
            check_instance(instance, &record.scope.instance_id)?;
            if let Some(action) = record.operation.strip_prefix("api.") {
                use crate::core::contracts::{
                    api::{ApiOperation, Recorded},
                    scalar::No,
                };
                let action: crate::core::input::ActionName =
                    action.parse().map_err(|_| AppError::storage())?;
                return Ok(Envelope::typed(Recorded {
                    request_id: record.request_id,
                    operation: ApiOperation::Call,
                    action: action.to_string(),
                    outcome: record.outcome,
                    result_available: No,
                })
                .scoped(record.scope));
            }
            let receipt = record
                .receipt
                .map(serde_json::from_value::<RecordedReceipt>)
                .transpose()
                .map_err(|_| AppError::storage())?;
            let data = MessageRequest {
                request_id: LocalId::new(&record.request_id)?,
                operation: record.operation,
                outcome: record.outcome,
                receipt,
            };
            Ok(Envelope::typed(data).scoped(record.scope))
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
    let creation = crate::napcat::runtime::instance_creation::checkpoint(store, refs, key)?;
    let dispatch = match crate::napcat::runtime::start_intent::get(store, refs, key) {
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
        .unwrap_or(&crate::napcat::runtime::start_intent::Outcome::Pending);
    let next = if matches!(state, "unregistered" | "changed") {
        NextAction::Instances {}
    } else {
        NextAction::Status {
            instance: instance_id.clone(),
        }
    };
    crate::core::deadline::check(deadline)?;
    Ok(Envelope::typed(StartRequest {
        request_id: LocalId::new(key)?,
        operation: StartOperation::Start,
        outcome: outcome.clone(),
        instance_id: LocalId::new(instance_id)?,
        run_id: LocalId::new(run_id)?,
        instance_state: state.into(),
        identity_verified: No,
        next_action: next,
    }))
}

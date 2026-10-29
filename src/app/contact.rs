//! Relationship mutations share the durable direct-call engine; reads project known fields.
mod read;
use crate::{
    cli::{args::ApiCallArgs, contact::ContactCommand},
    core::{
        contracts::{
            account::NumberOrString,
            contact::*,
            group::BoolOrString,
            scalar::{LocalId, No, Yes},
        },
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::time::Instant;
fn prepare(command: &ContactCommand) -> Result<Option<(&'static str, Value, Operation)>> {
    let (action, payload, operation) = match command {
        ContactCommand::Like { user_id, times, .. } => (
            "send_like",
            serde_json::to_value(LikePayload {
                user_id: user_id.to_string(),
                times: NumberOrString::Number(f64::from(times.get())),
            }),
            Operation::Like,
        ),
        ContactCommand::Remark {
            user_id, remark, ..
        } => (
            "set_friend_remark",
            serde_json::to_value(RemarkPayload {
                user_id: user_id.to_string(),
                remark: remark.to_string(),
            }),
            Operation::Remark,
        ),
        ContactCommand::Remove {
            user_id,
            block,
            both,
            ..
        } => (
            "delete_friend",
            serde_json::to_value(RemovePayload {
                friend_id: None,
                user_id: Some(NumberOrString::Text(user_id.to_string())),
                temp_block: Some(*block),
                temp_both_del: Some(*both),
            }),
            Operation::Remove,
        ),
        ContactCommand::Respond {
            flag,
            approve,
            remark,
            ..
        } => (
            "set_friend_add_request",
            serde_json::to_value(RespondPayload {
                flag: flag.to_string(),
                approve: Some(BoolOrString::Bool(*approve)),
                remark: remark.as_ref().map(ToString::to_string),
            }),
            Operation::Respond,
        ),
        ContactCommand::AcceptDoubt { flag, .. } => (
            "set_doubt_friends_add_request",
            serde_json::to_value(DoubtPayload {
                flag: flag.to_string(),
                approve: true,
            }),
            Operation::AcceptDoubt,
        ),
        _ => return Ok(None),
    };
    Ok(Some((
        action,
        payload.map_err(|_| AppError::invalid())?,
        operation,
    )))
}
pub(super) fn offline(command: &ContactCommand) -> Result<Option<Envelope>> {
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let (_, _, operation) = prepare(command)?.ok_or_else(AppError::invalid)?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &ContactCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Some((action, payload, operation)) = prepare(command)? else {
        return read::execute(command, session, deadline);
    };
    let w = command.write().ok_or_else(AppError::invalid)?;
    let args = ApiCallArgs {
        action: action.parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: w.request_id.clone(),
        dry_run: false,
        deadline: crate::cli::args::DeadlineArgs {
            timeout: w.deadline.timeout,
        },
    };
    let (result, scope) =
        super::api::execute_prepared(&args, payload, session, store, references, deadline)?
            .into_api_result()?;
    Ok(Some(
        Envelope::typed(Receipt {
            operation,
            request_id: LocalId::new(
                result
                    .request_id
                    .as_deref()
                    .ok_or_else(AppError::protocol)?,
            )?,
            outcome: result.outcome.ok_or_else(AppError::protocol)?,
            deduplicated: result.deduplicated,
        })
        .scoped(scope),
    ))
}

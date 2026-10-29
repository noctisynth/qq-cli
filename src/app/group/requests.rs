//! Explicit decisions share durable keys; numeric observations never bypass flag precision checks.
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        group::GroupCommand,
    },
    core::{
        contracts::{
            account::NumberOrString,
            group::BoolOrString,
            group_requests::*,
            scalar::{LocalId, No, Yes},
        },
        domain::{Conversation, identifier},
        error::{AppError, Result},
        input::DecimalId,
        output::Envelope,
    },
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::{Backend, parse_raw_envelope},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::time::Instant;
pub(super) fn offline(command: &GroupCommand) -> Result<Option<Envelope>> {
    if let GroupCommand::Respond { write, .. } = command
        && write.dry_run
    {
        return Ok(Some(Envelope::typed(Draft {
            operation: Operation::Respond,
            dry_run: Yes,
            identity_verified: No,
        })));
    }
    Ok(None)
}
fn opt<T: DeserializeOwned>(row: &Value, key: &str) -> Result<Option<T>> {
    match row.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => serde_json::from_value(value.clone())
            .map(Some)
            .map_err(|_| AppError::protocol()),
    }
}
fn id(row: &Value, key: &str) -> Option<DecimalId> {
    row.get(key)
        .and_then(identifier)
        .and_then(|v| v.parse().ok())
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or_else(AppError::protocol)
}
pub(super) fn execute<B: Backend>(
    command: &GroupCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if let GroupCommand::Respond {
        flag,
        approve,
        reason,
        search_limit,
        write,
    } = command
    {
        let payload = serde_json::to_value(RespondPayload {
            flag: flag.to_string(),
            approve: Some(BoolOrString::Bool(*approve)),
            reason: reason.as_ref().map(|v| Some(v.to_string())),
            count: Some(f64::from(search_limit.get())),
        })
        .map_err(|_| AppError::invalid())?;
        let args = ApiCallArgs {
            action: "set_group_add_request"
                .parse()
                .map_err(|_| AppError::protocol())?,
            params_file: None,
            stdin: false,
            request_id: write.request_id.clone(),
            dry_run: false,
            deadline: DeadlineArgs {
                timeout: write.deadline.timeout,
            },
        };
        let (result, scope) = super::super::api::execute_prepared(
            &args, payload, session, store, references, deadline,
        )?
        .into_api_result()?;
        return Ok(Some(
            Envelope::typed(Receipt {
                operation: Operation::Respond,
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
        ));
    }
    let GroupCommand::Requests {
        group_id,
        source,
        limit,
        ..
    } = command
    else {
        return Ok(None);
    };
    let (action, payload, source, source_batch_size) = match source.as_str() {
        "system" => (
            "get_group_system_msg",
            serde_json::to_value(BatchPayload {
                count: NumberOrString::Number(f64::from(limit.get())),
            }),
            Source::System,
            usize::from(limit.get()),
        ),
        "legacy-notices" => (
            "get_group_ignored_notifies",
            serde_json::to_value(EmptyPayload {}),
            Source::LegacyNotices,
            50,
        ),
        "doubt" => (
            "get_group_ignore_add_request",
            serde_json::to_value(EmptyPayload {}),
            Source::Doubt,
            10,
        ),
        _ => return Err(AppError::invalid()),
    };
    let payload = payload.map_err(|_| AppError::invalid())?;
    let target = group_id
        .as_ref()
        .map(|id| Conversation::Group(id.to_string()));
    let (value, scope) = session.read(target.as_ref(), deadline, |backend| {
        parse_raw_envelope(backend.call_raw(
            &action.parse().map_err(|_| AppError::protocol())?,
            &payload,
            deadline,
        )?)
    })?;
    let mut rows = Vec::new();
    match source {
        Source::Doubt => {
            for row in array(&value)? {
                rows.push((Kind::Join, row));
            }
        }
        _ => {
            let invites = value
                .get("invited_requests")
                .or_else(|| value.get("InvitedRequest"))
                .ok_or_else(AppError::protocol)?;
            for row in array(invites)? {
                rows.push((Kind::Invite, row));
            }
            for row in array(value.get("join_requests").ok_or_else(AppError::protocol)?)? {
                rows.push((Kind::Join, row));
            }
        }
    }
    if rows.len() > session.config.scan_limit {
        return Err(AppError::new(
            "SCAN_LIMIT",
            "Group request snapshot exceeds scanLimit.",
            4,
        ));
    }
    let targets = &session.config.permissions.targets;
    let filtered_by_policy = !targets.is_empty();
    let mut requests = Vec::new();
    let mut truncated = false;
    for (kind, row) in rows {
        if !row.is_object() {
            return Err(AppError::protocol());
        }
        let actual_group = id(row, "group_id");
        if group_id
            .as_ref()
            .is_some_and(|expected| actual_group.as_ref() != Some(expected))
        {
            continue;
        }
        if filtered_by_policy
            && !actual_group
                .as_ref()
                .is_some_and(|g| targets.contains(&format!("group:{g}")))
        {
            continue;
        }
        if requests.len() == usize::from(limit.get()) {
            truncated = true;
            continue;
        }
        let observed_request_id = row.get("request_id").and_then(identifier);
        let request_flag = row
            .get("request_id")
            .and_then(Value::as_u64)
            .filter(|id| *id > 0 && *id <= 9_007_199_254_740_991)
            .map(|id| id.to_string());
        requests.push(Request {
            kind,
            observed_request_id,
            request_id_exact: request_flag.is_some(),
            request_flag,
            group_id: actual_group,
            group_name: opt(row, "group_name")?,
            user_id: id(row, "invitor_uin"),
            nickname: opt(row, "requester_nick")?.or(opt(row, "invitor_nick")?),
            message: opt(row, "message")?,
            checked: opt(row, "checked")?,
            actor_id: id(row, "actor"),
        });
    }
    Ok(Some(
        Envelope::typed(Batch {
            source,
            requests,
            requested_limit: usize::from(limit.get()),
            source_batch_size,
            filtered_by_policy,
            truncated,
            complete: No,
        })
        .scoped(scope),
    ))
}

//! Reference actions validate inside a fresh durable intent, never on cached replay.
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs, MessageCommand},
        message_ops as selectors,
    },
    core::{
        contracts::{
            account::NumberOrString,
            message_ops::*,
            scalar::{LocalId, No, Yes},
        },
        domain::Conversation,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{
        message::{self, Locator},
        session::Session,
    },
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::time::Instant;
struct Prepared {
    action: &'static str,
    payload: Value,
    operation: Operation,
    conversation: Option<Conversation>,
    locator: Option<Locator>,
    all: bool,
}
fn prepare(command: &MessageCommand, references: Option<&References>) -> Result<Option<Prepared>> {
    match command {
        MessageCommand::Recall { message_ref, .. } => {
            let locator =
                message::load_locator(references.ok_or_else(AppError::storage)?, message_ref)?;
            Ok(Some(Prepared {
                action: "delete_msg",
                payload: serde_json::to_value(MessagePayload {
                    message_id: NumberOrString::Text(locator.backend_id.clone()),
                })
                .map_err(|_| AppError::invalid())?,
                operation: Operation::Recall,
                conversation: Some(locator.conversation.clone()),
                locator: Some(locator),
                all: false,
            }))
        }
        MessageCommand::MarkRead {
            conversation,
            message_ref,
            all,
            ..
        } => {
            let locator = message_ref
                .as_ref()
                .map(|token| {
                    message::load_locator(references.ok_or_else(AppError::storage)?, token)
                })
                .transpose()?;
            let conversation = locator
                .as_ref()
                .map(|l| l.conversation.clone())
                .or_else(|| conversation.clone());
            let (action, payload) = if *all {
                ("_mark_all_as_read", serde_json::to_value(EmptyPayload {}))
            } else if let Some(locator) = &locator {
                (
                    "mark_msg_as_read",
                    serde_json::to_value(MarkReadPayload {
                        user_id: None,
                        group_id: None,
                        message_id: Some(locator.backend_id.clone()),
                    }),
                )
            } else {
                let target = conversation.as_ref().ok_or_else(AppError::invalid)?;
                match target {
                    Conversation::Group(id) => (
                        "mark_group_msg_as_read",
                        serde_json::to_value(MarkReadPayload {
                            user_id: None,
                            group_id: Some(id.clone()),
                            message_id: None,
                        }),
                    ),
                    Conversation::Private(id) => (
                        "mark_private_msg_as_read",
                        serde_json::to_value(MarkReadPayload {
                            user_id: Some(NumberOrString::Text(id.clone())),
                            group_id: None,
                            message_id: None,
                        }),
                    ),
                }
            };
            Ok(Some(Prepared {
                action,
                payload: payload.map_err(|_| AppError::invalid())?,
                operation: Operation::MarkRead,
                conversation,
                locator,
                all: *all,
            }))
        }
        _ => Ok(None),
    }
}
fn draft(p: Prepared) -> Envelope {
    Envelope::typed(Draft {
        operation: p.operation,
        dry_run: Yes,
        identity_verified: No,
        conversation: p.conversation,
        all_conversations: p.all,
    })
}
pub(super) fn offline(command: &MessageCommand) -> Result<Option<Envelope>> {
    if let MessageCommand::MarkRead {
        message_ref: None,
        write,
        ..
    } = command
        && write.dry_run
    {
        return prepare(command, None).map(|p| p.map(draft));
    }
    Ok(None)
}
pub(super) fn execute<B: Backend>(
    command: &MessageCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Some(p) = prepare(command, references)? else {
        return Ok(None);
    };
    let write = selectors::write(command).ok_or_else(AppError::invalid)?;
    if write.dry_run {
        if let Some(locator) = &p.locator {
            message::validate_locator(locator, session)?;
        }
        return Ok(Some(draft(p)));
    }
    let args = ApiCallArgs {
        action: p.action.parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: write.request_id.clone(),
        dry_run: false,
        deadline: DeadlineArgs {
            timeout: write.deadline.timeout,
        },
    };
    let response = if let Some(locator) = &p.locator {
        let references = references.ok_or_else(AppError::storage)?;
        super::api::execute_guarded(
            &args,
            p.payload,
            session,
            store,
            Some(references),
            deadline,
            Some(super::api::Guard {
                targets: vec![locator.conversation.clone()],
                binding: serde_json::to_value(locator).map_err(|_| AppError::invalid())?,
                validate: || message::fetch(locator, session, references, deadline).map(|_| ()),
            }),
        )?
    } else {
        super::api::execute_prepared(&args, p.payload, session, store, references, deadline)?
    };
    let (result, scope) = response.into_api_result()?;
    Ok(Some(
        Envelope::typed(Receipt {
            operation: p.operation,
            request_id: LocalId::new(
                result
                    .request_id
                    .as_deref()
                    .ok_or_else(AppError::protocol)?,
            )?,
            outcome: result.outcome.ok_or_else(AppError::protocol)?,
            deduplicated: result.deduplicated,
            conversation: p.conversation,
            all_conversations: p.all,
        })
        .scoped(scope),
    ))
}

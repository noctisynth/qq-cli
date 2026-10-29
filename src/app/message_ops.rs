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
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;
struct Prepared {
    action: &'static str,
    payload: Value,
    operation: Operation,
    conversation: Option<Conversation>,
    locators: Vec<Locator>,
    all: bool,
    merged: Option<bool>,
    self_copies: Option<bool>,
}
fn encode(value: impl Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|_| AppError::invalid())
}
fn load(references: Option<&References>, token: &str) -> Result<Locator> {
    message::load_locator(references.ok_or_else(AppError::storage)?, token)
}
fn prepare(command: &MessageCommand, references: Option<&References>) -> Result<Option<Prepared>> {
    match command {
        MessageCommand::Forward {
            message_ref,
            conversation,
            merged,
            allow_self_copy,
            ..
        } => {
            let locators = message_ref
                .iter()
                .map(|token| load(references, token))
                .collect::<Result<Vec<_>>>()?;
            let first = locators.first().ok_or_else(AppError::invalid)?;
            if locators
                .iter()
                .any(|l| l.scope != first.scope || l.run_id != first.run_id)
            {
                return Err(AppError::new(
                    "FORWARD_SOURCE_MISMATCH",
                    "A forward batch must use the same source account, instance and run.",
                    4,
                ));
            }
            let self_copies = locators
                .iter()
                .any(|l| l.conversation != first.conversation);
            if self_copies && !*allow_self_copy {
                return Err(AppError::new(
                    "FORWARD_SELF_COPY_REQUIRED",
                    "Merging different conversations may send copies to your own chat; use --allow-self-copy only when authorized.",
                    4,
                ));
            }
            let merged = *merged || locators.len() > 1;
            let (group_id, user_id) = match conversation {
                Conversation::Group(id) => (Some(id.clone()), None),
                Conversation::Private(id) => (None, Some(id.clone())),
            };
            let (action, payload) = if merged {
                let nodes = locators
                    .iter()
                    .map(|l| ReferencedNode {
                        kind: ReferenceKind::Node,
                        data: ReferenceData {
                            id: l.backend_id.clone(),
                            nickname: String::new(),
                            content: [],
                        },
                    })
                    .collect();
                (
                    if conversation.is_group() {
                        "send_group_forward_msg"
                    } else {
                        "send_private_forward_msg"
                    },
                    encode(ForwardBundlePayload {
                        group_id,
                        user_id,
                        message: nodes,
                    })?,
                )
            } else {
                (
                    if conversation.is_group() {
                        "forward_group_single_msg"
                    } else {
                        "forward_friend_single_msg"
                    },
                    encode(ForwardPayload {
                        message_id: NumberOrString::Text(first.backend_id.clone()),
                        group_id,
                        user_id,
                    })?,
                )
            };
            Ok(Some(Prepared {
                action,
                payload,
                operation: Operation::Forward,
                conversation: Some(conversation.clone()),
                locators,
                all: false,
                merged: Some(merged),
                self_copies: Some(self_copies),
            }))
        }
        MessageCommand::Transcribe { message_ref, .. } => {
            let locator = load(references, message_ref)?;
            Ok(Some(Prepared {
                action: "fetch_ptt_text",
                payload: encode(crate::core::contracts::voice::TextPayload {
                    message_id: NumberOrString::Text(locator.backend_id.clone()),
                })?,
                operation: Operation::Transcribe,
                conversation: Some(locator.conversation.clone()),
                locators: vec![locator],
                all: false,
                merged: None,
                self_copies: None,
            }))
        }
        MessageCommand::Recall { message_ref, .. } => {
            let locator = load(references, message_ref)?;
            Ok(Some(Prepared {
                action: "delete_msg",
                payload: encode(MessagePayload {
                    message_id: NumberOrString::Text(locator.backend_id.clone()),
                })?,
                operation: Operation::Recall,
                conversation: Some(locator.conversation.clone()),
                locators: vec![locator],
                all: false,
                merged: None,
                self_copies: None,
            }))
        }
        MessageCommand::MarkRead {
            conversation,
            message_ref,
            all,
            ..
        } => {
            let locators = message_ref
                .as_ref()
                .map(|token| load(references, token))
                .transpose()?
                .into_iter()
                .collect::<Vec<_>>();
            let conversation = locators
                .first()
                .map(|l| l.conversation.clone())
                .or_else(|| conversation.clone());
            let (action, payload) = if *all {
                ("_mark_all_as_read", encode(EmptyPayload {})?)
            } else if let Some(locator) = locators.first() {
                (
                    "mark_msg_as_read",
                    encode(MarkReadPayload {
                        user_id: None,
                        group_id: None,
                        message_id: Some(locator.backend_id.clone()),
                    })?,
                )
            } else {
                let target = conversation.as_ref().ok_or_else(AppError::invalid)?;
                match target {
                    Conversation::Group(id) => (
                        "mark_group_msg_as_read",
                        encode(MarkReadPayload {
                            user_id: None,
                            group_id: Some(id.clone()),
                            message_id: None,
                        })?,
                    ),
                    Conversation::Private(id) => (
                        "mark_private_msg_as_read",
                        encode(MarkReadPayload {
                            user_id: Some(NumberOrString::Text(id.clone())),
                            group_id: None,
                            message_id: None,
                        })?,
                    ),
                }
            };
            Ok(Some(Prepared {
                action,
                payload,
                operation: Operation::MarkRead,
                conversation,
                locators,
                all: *all,
                merged: None,
                self_copies: None,
            }))
        }
        _ => Ok(None),
    }
}
fn draft(p: Prepared) -> Envelope {
    let source_messages = matches!(p.operation, Operation::Forward).then_some(p.locators.len());
    Envelope::typed(Draft {
        operation: p.operation,
        dry_run: Yes,
        identity_verified: No,
        conversation: p.conversation,
        all_conversations: p.all,
        merged: p.merged,
        may_send_self_copies: p.self_copies,
        source_messages,
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
        for locator in &p.locators {
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
    let source_messages = matches!(p.operation, Operation::Forward).then_some(p.locators.len());
    let response = if !p.locators.is_empty() {
        let references = references.ok_or_else(AppError::storage)?;
        let mut targets = p
            .locators
            .iter()
            .map(|l| l.conversation.clone())
            .collect::<Vec<_>>();
        if let Some(target) = &p.conversation {
            targets.push(target.clone());
        }
        if p.self_copies == Some(true) {
            targets.push(Conversation::Private(p.locators[0].scope.account.clone()));
        }
        // Keep the original recall/read fingerprint stable; forward batches bind all locators.
        let binding = if matches!(p.operation, Operation::Forward) {
            encode(&p.locators)?
        } else {
            encode(&p.locators[0])?
        };
        super::api::execute_guarded(
            &args,
            p.payload,
            session,
            store,
            Some(references),
            deadline,
            Some(super::api::Guard {
                receipt_target: if p.merged == Some(true) {
                    p.conversation.clone()
                } else {
                    None
                },
                targets,
                binding,
                validate: || {
                    for locator in &p.locators {
                        let message = message::fetch(locator, session, references, deadline)?;
                        if matches!(p.operation, Operation::Transcribe)
                            && !message.value.segments.iter().any(|s| {
                                matches!(
                                    s,
                                    crate::core::contracts::message::Segment::Known(
                                        crate::core::contracts::message::KnownSegment::Record { .. }
                                    )
                                )
                            })
                        {
                            return Err(AppError::new(
                                "VOICE_REQUIRED",
                                "The verified message does not contain a record segment.",
                                4,
                            ));
                        }
                    }
                    Ok(())
                },
            }),
        )?
    } else {
        super::api::execute_prepared(&args, p.payload, session, store, references, deadline)?
    };
    let (result, scope) = response.into_api_result()?;
    if matches!(p.operation, Operation::Transcribe) {
        let text = result
            .result
            .map(|document| document.decode_data::<crate::core::contracts::voice::TextResult>())
            .transpose()?
            .map(|r| r.text);
        return Ok(Some(
            Envelope::typed(crate::core::contracts::voice::Transcript {
                request_id: LocalId::new(
                    result
                        .request_id
                        .as_deref()
                        .ok_or_else(AppError::protocol)?,
                )?,
                outcome: result.outcome.ok_or_else(AppError::protocol)?,
                deduplicated: result.deduplicated,
                conversation: p.conversation.ok_or_else(AppError::protocol)?,
                text_available: text.is_some(),
                text,
            })
            .scoped(scope),
        ));
    }
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
            merged: p.merged,
            may_send_self_copies: p.self_copies,
            source_messages,
            message: result.receipt,
        })
        .scoped(scope),
    ))
}

//! Flash tasks use persistent owned inputs; acknowledgements never imply transfer completion.
mod read;
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        flash::FlashCommand,
    },
    core::{
        contracts::{
            flash::*,
            scalar::{LocalId, No, Yes},
        },
        domain::Conversation,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{attachments, session::Session},
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::time::Instant;
fn prepare(
    command: &FlashCommand,
    paths: Vec<String>,
) -> Result<Option<(&'static str, Value, Operation)>> {
    let (action, payload, operation) = match command {
        FlashCommand::Create(args) => {
            if paths.is_empty() || paths.len() > 32 {
                return Err(AppError::invalid());
            }
            (
                "create_flash_task",
                serde_json::to_value(CreatePayload {
                    files: Paths::Many(paths),
                    name: args.name.as_ref().map(ToString::to_string),
                    thumb_path: None,
                    validity_seconds: Some(f64::from(args.validity_days.seconds())),
                }),
                Operation::Create,
            )
        }
        FlashCommand::Send {
            fileset_id,
            conversation,
            ..
        } => (
            "send_flash_msg",
            serde_json::to_value(SendPayload {
                fileset_id: fileset_id.to_string(),
                user_id: if let Conversation::Private(id) = conversation {
                    Some(id.clone())
                } else {
                    None
                },
                group_id: if let Conversation::Group(id) = conversation {
                    Some(id.clone())
                } else {
                    None
                },
            }),
            Operation::Send,
        ),
        FlashCommand::Download { fileset_id, .. } => (
            "download_fileset",
            serde_json::to_value(SetPayload {
                fileset_id: fileset_id.to_string(),
            }),
            Operation::Download,
        ),
        _ => return Ok(None),
    };
    let payload = payload.map_err(|_| AppError::invalid())?;
    if serde_json::to_vec(&payload)
        .map_err(|_| AppError::invalid())?
        .len()
        > crate::core::contracts::api::json::MAX_INPUT
    {
        return Err(AppError::invalid());
    }
    Ok(Some((action, payload, operation)))
}
pub(super) fn offline(command: &FlashCommand) -> Result<Option<Envelope>> {
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let paths = if let FlashCommand::Create(args) = command {
        let deadline =
            Instant::now() + std::time::Duration::from_secs(u64::from(args.write.deadline.timeout));
        for path in &args.paths {
            attachments::validate_resource(path, deadline)?;
        }
        args.paths.iter().map(|_| "managed-input".into()).collect()
    } else {
        vec![]
    };
    let (_, _, operation) = prepare(command, paths)?.ok_or_else(AppError::invalid)?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &FlashCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Some(write) = command.write() else {
        return read::execute(command, session, deadline).map(Some);
    };
    let target = if let FlashCommand::Send { conversation, .. } = command {
        Some(conversation.clone())
    } else {
        None
    };
    session.authorize(target.as_ref(), true)?;
    if matches!(command, FlashCommand::Download { .. }) {
        session.authorize(None, false)?;
        if !session.config.permissions.targets.is_empty() {
            return Err(AppError::new(
                "API_TARGET_SCOPE_UNKNOWN",
                "File-set downloads cannot prove original conversation scopes.",
                4,
            ));
        }
    }
    let snapshots = if let FlashCommand::Create(args) = command {
        args.paths
            .iter()
            .map(|path| {
                attachments::prepare_resource_for_run(path, store, session.instance, deadline)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };
    let paths = snapshots
        .iter()
        .map(|s| {
            s.path()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(AppError::invalid)
        })
        .collect::<Result<Vec<_>>>()?;
    let (action, payload, operation) = prepare(command, paths)?.ok_or_else(AppError::invalid)?;
    let args = ApiCallArgs {
        action: action.parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: write.request_id.clone(),
        dry_run: false,
        deadline: DeadlineArgs {
            timeout: write.deadline.timeout,
        },
    };
    let response = if snapshots.is_empty() {
        super::api::execute_prepared(&args, payload, session, store, references, deadline)
    } else {
        super::api::execute_files(
            &args,
            (payload, snapshots),
            session,
            store,
            references,
            deadline,
        )
    }?;
    let (result, scope) = response.into_api_result()?;
    let created = if matches!(operation, Operation::Create) {
        result
            .result
            .map(|d| d.decode_data::<Created>().map(|d| d.result))
            .transpose()?
    } else {
        None
    };
    let (fileset_id, share_link, native_expiry) = created
        .map(|s| {
            (
                Some(s.file_set_id),
                s.share_link.filter(|s| !s.is_empty()),
                s.expire_time,
            )
        })
        .unwrap_or((None, None, None));
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
            result_available: result.result_available,
            transfer_verified: No,
            fileset_id,
            share_link,
            native_expiry,
            conversation: target,
        })
        .scoped(scope),
    ))
}

//! Private online-file tasks share explicit native peer IDs and once-only intents.
mod read;
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        online::OnlineCommand,
    },
    core::{
        contracts::{
            online::*,
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
fn native_id(text: &str) -> Result<String> {
    if text.starts_with("m1.") || text.starts_with("f1.") {
        return Err(AppError::invalid());
    }
    Ok(text.to_owned())
}
fn prepare(command: &OnlineCommand, path: String) -> Result<(&'static str, Value, Operation)> {
    let user_id = command.user().to_string();
    let (action, value, op) = match command {
        OnlineCommand::Send(a) => (
            "send_online_file",
            serde_json::to_value(SendPayload {
                user_id,
                file_path: path,
                file_name: a.name.as_ref().map(ToString::to_string),
            }),
            Operation::Send,
        ),
        OnlineCommand::SendFolder(a) => (
            "send_online_folder",
            serde_json::to_value(FolderPayload {
                user_id,
                folder_path: path,
                folder_name: a.name.as_ref().map(ToString::to_string),
            }),
            Operation::SendFolder,
        ),
        OnlineCommand::Receive(a) | OnlineCommand::Refuse(a) => (
            if matches!(command, OnlineCommand::Receive(_)) {
                "receive_online_file"
            } else {
                "refuse_online_file"
            },
            serde_json::to_value(ElementPayload {
                user_id,
                msg_id: native_id(&a.native_msg_id)?,
                element_id: native_id(&a.element_id)?,
            }),
            if matches!(command, OnlineCommand::Receive(_)) {
                Operation::Receive
            } else {
                Operation::Refuse
            },
        ),
        OnlineCommand::Cancel { native_msg_id, .. } => (
            "cancel_online_file",
            serde_json::to_value(CancelPayload {
                user_id,
                msg_id: native_id(native_msg_id)?,
            }),
            Operation::Cancel,
        ),
        _ => return Err(AppError::invalid()),
    };
    Ok((action, value.map_err(|_| AppError::invalid())?, op))
}
fn validate_input(command: &OnlineCommand, deadline: Instant) -> Result<()> {
    match command {
        OnlineCommand::Send(a) => attachments::validate_source(&a.path),
        OnlineCommand::SendFolder(a) => {
            if !a.path.is_dir() {
                return Err(AppError::invalid());
            }
            attachments::validate_resource(&a.path, deadline)
        }
        _ => Ok(()),
    }
}
pub(super) fn offline(command: &OnlineCommand) -> Result<Option<Envelope>> {
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let deadline = Instant::now() + std::time::Duration::from_secs(u64::from(command.timeout()));
    validate_input(command, deadline)?;
    let (_, _, operation) = prepare(command, "managed-input".into())?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &OnlineCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Some(write) = command.write() else {
        return read::execute(command, session, deadline).map(Some);
    };
    let target = Conversation::Private(command.user().to_string());
    session.authorize(Some(&target), true)?;
    if matches!(command, OnlineCommand::Receive(_)) {
        session.authorize(Some(&target), false)?;
        if !session.config.permissions.media_download {
            return Err(AppError::new(
                "PERMISSION_DENIED",
                "The local policy does not permit receiving online files.",
                4,
            ));
        }
    }
    validate_input(command, deadline)?;
    let snapshot = match command {
        OnlineCommand::Send(a) | OnlineCommand::SendFolder(a) => Some(
            attachments::prepare_resource_for_run(&a.path, store, session.instance, deadline)?,
        ),
        _ => None,
    };
    let path = snapshot
        .as_ref()
        .map(|s| {
            s.path()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(AppError::invalid)
        })
        .transpose()?
        .unwrap_or_default();
    let (action, payload, operation) = prepare(command, path)?;
    let args = ApiCallArgs {
        action: action.parse().map_err(|_| AppError::invalid())?,
        params_file: None,
        stdin: false,
        request_id: write.request_id.clone(),
        dry_run: false,
        deadline: DeadlineArgs {
            timeout: write.deadline.timeout,
        },
    };
    let result = if let Some(snapshot) = snapshot {
        super::api::execute_file(
            &args,
            (payload, snapshot),
            session,
            store,
            references,
            deadline,
        )
    } else {
        super::api::execute_prepared(&args, payload, session, store, references, deadline)
    }?;
    let (result, scope) = result.into_api_result()?;
    let sent = if matches!(operation, Operation::Send | Operation::SendFolder) {
        result.result.map(|d| d.decode_data::<Sent>()).transpose()?
    } else {
        None
    };
    let (native_msg_id, element_id) = sent
        .map(|s| (Some(s.msg_id), Some(s.element_id)))
        .unwrap_or((None, None));
    Ok(Some(
        Envelope::typed(Receipt {
            operation,
            request_id: LocalId::new(
                result
                    .request_id
                    .as_deref()
                    .ok_or_else(AppError::protocol)?,
            )?,
            conversation: target,
            outcome: result.outcome.ok_or_else(AppError::protocol)?,
            deduplicated: result.deduplicated,
            result_available: result.result_available,
            transfer_verified: No,
            native_msg_id,
            element_id,
        })
        .scoped(scope),
    ))
}

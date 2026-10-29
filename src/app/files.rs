//! File mutations and explicit uploads share the durable API ledger.
mod read;
mod stage;
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        files::FileCommand,
    },
    core::{
        contracts::{
            files::*,
            scalar::{LocalId, No, Yes},
        },
        domain::{Conversation, is_exact_js_id},
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{attachments, session::Session},
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::time::Instant;
fn encode(value: impl serde::Serialize) -> Result<Value> {
    let value = serde_json::to_value(value).map_err(|_| AppError::invalid())?;
    if serde_json::to_vec(&value)
        .map_err(|_| AppError::invalid())?
        .len()
        > crate::core::contracts::api::json::MAX_INPUT
    {
        return Err(AppError::invalid());
    }
    Ok(value)
}
fn prepare(
    command: &FileCommand,
    path: Option<(&str, &str)>,
) -> Result<Option<(&'static str, Value, Operation)>> {
    Ok(Some(match command {
        FileCommand::Mkdir { group_id, name, .. } => (
            "create_group_file_folder",
            encode(MkdirPayload {
                group_id: group_id.to_string(),
                folder_name: Some(name.to_string()),
                name: None,
            })?,
            Operation::Mkdir,
        ),
        FileCommand::Rmdir {
            group_id,
            folder_id,
            ..
        } => (
            "delete_group_folder",
            encode(FolderPayload {
                group_id: group_id.to_string(),
                folder: None,
                folder_id: Some(folder_id.to_string()),
            })?,
            Operation::Rmdir,
        ),
        FileCommand::Remove {
            group_id, file_id, ..
        } => (
            "delete_group_file",
            encode(GroupFilePayload {
                group_id: group_id.to_string(),
                file_id: file_id.to_string(),
            })?,
            Operation::Remove,
        ),
        FileCommand::Transfer {
            group_id, file_id, ..
        } => (
            "trans_group_file",
            encode(GroupFilePayload {
                group_id: group_id.to_string(),
                file_id: file_id.to_string(),
            })?,
            Operation::Transfer,
        ),
        FileCommand::Move {
            group_id,
            file_id,
            from,
            to,
            ..
        } => {
            if !is_exact_js_id(group_id) {
                return Err(AppError::invalid());
            }
            (
                "move_group_file",
                encode(MovePayload {
                    group_id: group_id.to_string(),
                    file_id: file_id.to_string(),
                    current_parent_directory: from.to_string(),
                    target_parent_directory: to.to_string(),
                })?,
                Operation::Move,
            )
        }
        FileCommand::Rename {
            group_id,
            file_id,
            name,
            folder,
            ..
        } => {
            if !is_exact_js_id(group_id) {
                return Err(AppError::invalid());
            }
            (
                "rename_group_file",
                encode(RenamePayload {
                    group_id: group_id.to_string(),
                    file_id: file_id.to_string(),
                    current_parent_directory: folder.to_string(),
                    new_name: name.to_string(),
                })?,
                Operation::Rename,
            )
        }
        FileCommand::Upload(args) => {
            let (file, default_name) = path.ok_or_else(AppError::invalid)?;
            let name = args.name.as_deref().unwrap_or(default_name).to_owned();
            match &args.conversation {
                Conversation::Private(user) => {
                    if args.folder.is_some() {
                        return Err(AppError::invalid());
                    }
                    (
                        "upload_private_file",
                        encode(PrivateUploadPayload {
                            user_id: user.clone(),
                            file: file.into(),
                            name,
                            upload_file: true,
                        })?,
                        Operation::Upload,
                    )
                }
                Conversation::Group(group) => (
                    "upload_group_file",
                    encode(GroupUploadPayload {
                        group_id: group.clone(),
                        file: file.into(),
                        name,
                        folder: args.folder.as_ref().map(ToString::to_string),
                        folder_id: None,
                        upload_file: true,
                    })?,
                    Operation::Upload,
                ),
            }
        }
        _ => return Ok(None),
    }))
}
pub(super) fn offline(command: &FileCommand) -> Result<Option<Envelope>> {
    if let Some(result) = stage::offline(command)? {
        return Ok(Some(result));
    }
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let path = if let FileCommand::Upload(args) = command {
        attachments::validate_source(&args.file)?;
        Some((
            "managed-input",
            args.file
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or_else(AppError::invalid)?,
        ))
    } else {
        None
    };
    let (_, _, operation) = prepare(command, path)?.ok_or_else(AppError::invalid)?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        conversation: command.target().ok_or_else(AppError::invalid)?,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &FileCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if let Some(result) = stage::execute(command, session, store, references, deadline)? {
        return Ok(Some(result));
    }
    let Some(write) = command.write() else {
        return read::execute(command, session, references, deadline).map(Some);
    };
    let target = command.target().ok_or_else(AppError::invalid)?;
    session.authorize(Some(&target), true)?;
    if let FileCommand::Upload(a) = command
        && !a.conversation.is_group()
        && a.folder.is_some()
    {
        return Err(AppError::invalid());
    }
    let snapshot = if let FileCommand::Upload(a) = command {
        Some(attachments::prepare_for_run(
            &a.file,
            attachments::Kind::File,
            store,
            session.instance,
            deadline,
        )?)
    } else {
        None
    };
    let name = snapshot.as_ref().map(|s| s.fingerprint().name);
    let path = snapshot
        .as_ref()
        .map(|s| s.path().to_str().ok_or_else(AppError::invalid))
        .transpose()?;
    let (action, payload, operation) =
        prepare(command, path.zip(name.as_deref()))?.ok_or_else(AppError::invalid)?;
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
    let response = if let Some(snapshot) = snapshot {
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
    let (result, scope) = response.into_api_result()?;
    let native_file_id = if matches!(operation, Operation::Upload) {
        result
            .result
            .map(|d| d.decode_data::<Uploaded>().map(|p| p.file_id))
            .transpose()?
            .flatten()
    } else {
        None
    };
    Ok(Some(
        Envelope::typed(Receipt {
            operation,
            conversation: target,
            request_id: LocalId::new(
                result
                    .request_id
                    .as_deref()
                    .ok_or_else(AppError::protocol)?,
            )?,
            outcome: result.outcome.ok_or_else(AppError::protocol)?,
            deduplicated: result.deduplicated,
            file_id_available: native_file_id.is_some(),
            native_file_id,
        })
        .scoped(scope),
    ))
}

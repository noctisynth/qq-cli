//! Group album business routes. File preparation does not confer native source authority.
mod read;
use crate::{
    cli::{
        album::AlbumCommand,
        args::{ApiCallArgs, DeadlineArgs},
    },
    core::{
        contracts::{
            album::*,
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
use std::{path::Path, time::Instant};
fn files(command: &AlbumCommand) -> Vec<&Path> {
    match command {
        AlbumCommand::Upload(args) => args.file.iter().map(|p| p.as_path()).collect(),
        AlbumCommand::UploadVideo { file, .. } => vec![file.as_path()],
        _ => vec![],
    }
}
fn prepare(
    command: &AlbumCommand,
    paths: &[String],
) -> Result<Option<(&'static str, Value, Operation)>> {
    let (action, payload, operation) = match command {
        AlbumCommand::Remove {
            group_id,
            album_id,
            lloc,
            ..
        } => (
            "del_group_album_media",
            serde_json::to_value(RemovePayload {
                group_id: group_id.to_string(),
                album_id: album_id.to_string(),
                lloc: lloc.to_string(),
            }),
            Operation::Remove,
        ),
        AlbumCommand::Like {
            group_id,
            album_id,
            batch_id,
            enabled,
            lloc,
            ..
        } => (
            if *enabled {
                "set_group_album_media_like"
            } else {
                "cancel_group_album_media_like"
            },
            serde_json::to_value(LikePayload {
                group_id: group_id.to_string(),
                album_id: album_id.to_string(),
                batch_id: batch_id.to_string(),
                lloc: lloc.as_ref().map(ToString::to_string),
            }),
            Operation::Like,
        ),
        AlbumCommand::Comment {
            group_id,
            album_id,
            lloc,
            content,
            ..
        } => (
            "do_group_album_comment",
            serde_json::to_value(CommentPayload {
                group_id: group_id.to_string(),
                album_id: album_id.to_string(),
                lloc: lloc.to_string(),
                content: content.to_string(),
            }),
            Operation::Comment,
        ),
        AlbumCommand::Upload(args) => {
            if args.file.is_empty() || args.file.len() > 32 || paths.len() != args.file.len() {
                return Err(AppError::invalid());
            }
            if paths.len() == 1 {
                (
                    "upload_image_to_qun_album",
                    serde_json::to_value(UploadPayload {
                        group_id: args.group_id.to_string(),
                        album_id: args.album_id.to_string(),
                        album_name: args.album_name.to_string(),
                        file: paths[0].clone(),
                    }),
                    Operation::Upload,
                )
            } else {
                (
                    "upload_images_to_qun_album",
                    serde_json::to_value(UploadManyPayload {
                        group_id: args.group_id.to_string(),
                        album_id: args.album_id.to_string(),
                        album_name: args.album_name.to_string(),
                        files: paths.to_vec(),
                    }),
                    Operation::Upload,
                )
            }
        }
        AlbumCommand::UploadVideo {
            group_id,
            album_id,
            album_name,
            ..
        } => (
            "upload_video_to_qun_album",
            serde_json::to_value(UploadPayload {
                group_id: group_id.to_string(),
                album_id: album_id.to_string(),
                album_name: album_name.to_string(),
                file: paths.first().ok_or_else(AppError::invalid)?.clone(),
            }),
            Operation::Video,
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
pub(super) fn offline(command: &AlbumCommand) -> Result<Option<Envelope>> {
    if !command.write().is_some_and(|w| w.dry_run) {
        return Ok(None);
    }
    let inputs = files(command);
    for path in &inputs {
        attachments::validate_source(path)?;
    }
    let paths = inputs
        .iter()
        .map(|_| "managed-input".into())
        .collect::<Vec<_>>();
    let (_, _, operation) = prepare(command, &paths)?.ok_or_else(AppError::invalid)?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        group_id: command.group().clone(),
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &AlbumCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Some(write) = command.write() else {
        return read::execute(command, session, deadline).map(Some);
    };
    let group = command.group();
    let target = Conversation::Group(group.to_string());
    session.authorize(Some(&target), true)?;
    let snapshots = files(command)
        .into_iter()
        .map(|path| {
            attachments::prepare_for_run(
                path,
                if matches!(command, AlbumCommand::UploadVideo { .. }) {
                    attachments::Kind::File
                } else {
                    attachments::Kind::Image
                },
                store,
                session.instance,
                deadline,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let paths = snapshots
        .iter()
        .map(|file| {
            file.path()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(AppError::invalid)
        })
        .collect::<Result<Vec<_>>>()?;
    let (action, payload, operation) = prepare(command, &paths)?.ok_or_else(AppError::invalid)?;
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
    let result = if snapshots.is_empty() {
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
    let (result, scope) = result.into_api_result()?;
    Ok(Some(
        Envelope::typed(Receipt {
            operation,
            group_id: group.clone(),
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

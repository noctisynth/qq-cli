//! QQ-space writes reuse the shared intent and never persist content or post bodies.
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        social::QzoneCommand,
    },
    core::{
        contracts::{
            account::NumberOrString,
            scalar::{LocalId, No, Yes},
            social::*,
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
fn prepare(command: &QzoneCommand) -> Result<(&'static str, Value, Operation)> {
    let (action, value, operation) = match command {
        QzoneCommand::Post(args) => {
            let targeted = matches!(args.audience, Audience::Selected | Audience::Exclude);
            if targeted == args.user_id.is_empty()
                || args.user_id.len() > 100
                || args.image.len() > 9
                || args
                    .user_id
                    .iter()
                    .any(|id| !crate::core::domain::is_exact_js_id(id))
            {
                return Err(AppError::invalid());
            }
            let images = args
                .image
                .iter()
                .map(|path| super::profile_image::read(path).map(|v| v.0))
                .collect::<Result<Vec<_>>>()?;
            (
                "send_qzone_msg",
                serde_json::to_value(PostPayload {
                    content: args.content.to_string(),
                    images: (!images.is_empty()).then_some(images),
                    ugc_right: Some(NumberOrString::Number(f64::from(args.audience.code()))),
                    target_uins: targeted.then(|| {
                        args.user_id
                            .iter()
                            .map(|id| NumberOrString::Text(id.to_string()))
                            .collect()
                    }),
                }),
                Operation::Post,
            )
        }
        QzoneCommand::Delete { tid, .. } => (
            "delete_qzone_msg",
            serde_json::to_value(DeletePayload {
                tid: tid.to_string(),
            }),
            Operation::Delete,
        ),
    };
    let value = value.map_err(|_| AppError::invalid())?;
    if serde_json::to_vec(&value)
        .map_err(|_| AppError::invalid())?
        .len()
        > 4 * 1024 * 1024
    {
        return Err(AppError::new(
            "INPUT_LIMIT",
            "QQ-space payload exceeds the 4 MiB transport budget.",
            2,
        ));
    }
    Ok((action, value, operation))
}
pub(super) fn offline(command: &QzoneCommand) -> Result<Option<Envelope>> {
    if !command.write().dry_run {
        return Ok(None);
    }
    let (_, _, operation) = prepare(command)?;
    Ok(Some(Envelope::typed(Draft {
        operation,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &QzoneCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    session.authorize(None, true)?;
    if !session.config.permissions.targets.is_empty() {
        return Err(AppError::new(
            "API_TARGET_SCOPE_UNKNOWN",
            "QQ-space audiences cannot prove the configured conversation target scope.",
            4,
        ));
    }
    let (action, payload, operation) = prepare(command)?;
    let write = command.write();
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
    let (result, scope) =
        super::api::execute_prepared(&args, payload, session, store, references, deadline)?
            .into_api_result()?;
    let tid = if matches!(operation, Operation::Post) {
        result
            .result
            .map(|d| d.decode_data::<Published>().map(|p| p.tid))
            .transpose()?
    } else {
        None
    };
    Ok(Envelope::typed(Receipt {
        operation,
        request_id: LocalId::new(
            result
                .request_id
                .as_deref()
                .ok_or_else(AppError::protocol)?,
        )?,
        outcome: result.outcome.ok_or_else(AppError::protocol)?,
        deduplicated: result.deduplicated,
        tid_available: tid.is_some(),
        tid,
    })
    .scoped(scope))
}

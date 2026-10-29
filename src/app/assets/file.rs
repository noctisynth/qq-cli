//! File-only native emoji import. Original paths never cross the API boundary.
use crate::{
    cli::{
        args::{ApiCallArgs, Command, DeadlineArgs},
        assets::{AddEmojiArgs, EmojiCommand},
    },
    core::{
        contracts::{
            account::NumberOrString,
            assets::*,
            scalar::{LocalId, No, Yes},
        },
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{attachments, session::Session},
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use std::time::Instant;
fn package(args: &AddEmojiArgs) -> Result<String> {
    args.package_id
        .as_ref()
        .map(|id| {
            crate::core::domain::parse_js_integer(id)
                .map(|v| v.to_string())
                .ok_or_else(|| {
                    AppError::new(
                        "PACKAGE_ID_RANGE",
                        "Package ID must be exactly representable by the upstream numeric route.",
                        2,
                    )
                })
        })
        .transpose()
        .map(|v| v.unwrap_or_else(|| "0".into()))
}
pub(super) fn offline(command: &Command) -> Result<Option<Envelope>> {
    let Command::Emoji {
        command: EmojiCommand::Add(args),
    } = command
    else {
        return Ok(None);
    };
    if !args.write.dry_run {
        return Ok(None);
    }
    package(args)?;
    attachments::validate_source(&args.file)?;
    Ok(Some(Envelope::typed(Draft {
        operation: Operation::Add,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &Command,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let Command::Emoji {
        command: EmojiCommand::Add(input),
    } = command
    else {
        return Ok(None);
    };
    session.authorize(None, true)?;
    let package = package(input)?;
    let snapshot = attachments::prepare_for_run(
        &input.file,
        attachments::Kind::Image,
        store,
        session.instance,
        deadline,
    )?;
    let fingerprint = snapshot.fingerprint();
    let payload = serde_json::to_value(AddPayload {
        file: snapshot
            .path()
            .to_str()
            .ok_or_else(AppError::invalid)?
            .into(),
        emoji_id: Some(NumberOrString::Text(
            input
                .emoji_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )),
        package_id: Some(NumberOrString::Text(package)),
        file_name: Some(fingerprint.name),
        file_size: Some(NumberOrString::Text(fingerprint.size_bytes.to_string())),
        md5: None,
        is_mark_face: Some(input.market),
        is_origin: Some(input.original),
    })
    .map_err(|_| AppError::invalid())?;
    let args = ApiCallArgs {
        action: "add_custom_face"
            .parse()
            .map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: input.write.request_id.clone(),
        dry_run: false,
        deadline: DeadlineArgs {
            timeout: input.write.deadline.timeout,
        },
    };
    let (result, scope) = crate::app::api::execute_file(
        &args,
        (payload, snapshot),
        session,
        store,
        references,
        deadline,
    )?
    .into_api_result()?;
    Ok(Some(
        Envelope::typed(Receipt {
            operation: Operation::Add,
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

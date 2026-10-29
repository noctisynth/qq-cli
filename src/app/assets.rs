//! Explicit own-account writes share the durable API engine; reads project bounded metadata.
mod file;
mod read;
use crate::{
    cli::{
        args::{ApiCallArgs, Command, DeadlineArgs},
        assets::{EmojiCommand, FavoriteCommand},
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
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use serde_json::Value;
use std::{io::Read, time::Instant};
fn input(
    raw: Option<&crate::core::input::TextValue>,
    file: Option<&std::path::Path>,
    stdin: bool,
) -> Result<String> {
    if let Some(raw) = raw {
        return Ok(raw.to_string());
    }
    let bad = || {
        AppError::new(
            "FAVORITE_INPUT",
            "Favorite input must be bounded UTF-8 from the explicit source.",
            2,
        )
    };
    let mut bytes = Vec::new();
    let limit = crate::core::contracts::api::json::MAX_INPUT;
    if let Some(path) = file {
        let file = std::fs::File::open(path).map_err(|_| bad())?;
        if !file.metadata().map_err(|_| bad())?.is_file() {
            return Err(bad());
        }
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| bad())?;
    } else if stdin {
        std::io::stdin()
            .lock()
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| bad())?;
    } else {
        return Err(AppError::invalid());
    }
    if bytes.len() > limit {
        return Err(bad());
    }
    String::from_utf8(bytes).map_err(|_| bad())
}
fn encode(value: impl serde::Serialize) -> Result<Value> {
    let value = serde_json::to_value(value).map_err(|_| AppError::invalid())?;
    if serde_json::to_vec(&value)
        .map_err(|_| AppError::invalid())?
        .len()
        > crate::core::contracts::api::json::MAX_INPUT
    {
        return Err(AppError::new(
            "INPUT_LIMIT",
            "Repository parameters exceed the 1 MiB JSON budget.",
            2,
        ));
    }
    Ok(value)
}
fn prepare(command: &Command) -> Result<Option<(&'static str, Value, Operation)>> {
    Ok(Some(match command {
        Command::Favorite {
            command:
                FavoriteCommand::Create {
                    brief,
                    raw_data,
                    raw_file,
                    stdin,
                    ..
                },
        } => (
            "create_collection",
            encode(CreatePayload {
                raw_data: input(raw_data.as_ref(), raw_file.as_deref(), *stdin)?,
                brief: brief.to_string(),
            })?,
            Operation::Create,
        ),
        Command::Emoji {
            command: EmojiCommand::Remove { res_id, .. },
        } => (
            "delete_custom_face",
            encode(RemovePayload {
                ids: Some(res_id.iter().map(ToString::to_string).collect()),
                res_id: None,
                id: None,
                md5: None,
            })?,
            Operation::Remove,
        ),
        Command::Emoji {
            command:
                EmojiCommand::Describe {
                    res_id,
                    emoji_id,
                    md5,
                    description,
                    ..
                },
        } => {
            let id = crate::core::domain::parse_js_integer(emoji_id).ok_or_else(|| {
                AppError::new(
                    "EMOJI_ID_RANGE",
                    "Emoji ID must be exactly representable by the upstream numeric route.",
                    2,
                )
            })?;
            (
                "set_custom_face_desc",
                encode(DescribePayload {
                    emoji_id: NumberOrString::Text(id.to_string()),
                    res_id: res_id.to_string(),
                    md5: md5.to_string(),
                    desc: description.to_string(),
                })?,
                Operation::Describe,
            )
        }
        _ => return Ok(None),
    }))
}
pub(super) fn offline(command: &Command) -> Result<Option<Envelope>> {
    if let Some(draft) = file::offline(command)? {
        return Ok(Some(draft));
    }
    if !crate::cli::assets::write(command).is_some_and(|w| w.dry_run) {
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
    command: &Command,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if let Some(result) = file::execute(command, session, store, references, deadline)? {
        return Ok(Some(result));
    }
    let Some((action, payload, operation)) = prepare(command)? else {
        return read::execute(command, session, deadline);
    };
    let write = crate::cli::assets::write(command).ok_or_else(AppError::invalid)?;
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

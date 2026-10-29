//! Route the typed domain tasks that share explicit backend actions.
use crate::{
    cli::args::Command,
    core::{error::Result, output::Envelope},
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use std::time::Instant;
pub(super) fn offline(command: &Command) -> Result<Option<Envelope>> {
    match command {
        Command::Qzone { command } => super::social::offline(command),
        Command::Favorite { .. } | Command::Emoji { .. } => super::assets::offline(command),
        Command::Message { command } => super::message_ops::offline(command),
        Command::Account { command } => super::account::offline(command),
        Command::Group { command } => super::group::offline(command),
        Command::Contact { command } => super::contact::offline(command),
        Command::Api { command } => super::api::offline(command),
        _ => Ok(None),
    }
}
pub(super) fn execute<B: Backend>(
    command: &Command,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let response = match command {
        Command::Qzone { command } => {
            super::social::execute(command, session, store, references, deadline).map(Some)
        }
        Command::Favorite { .. } | Command::Emoji { .. } => {
            super::assets::execute(command, session, store, references, deadline)
        }
        Command::Message { command } => {
            super::message_ops::execute(command, session, store, references, deadline)
        }
        Command::Account { command } => {
            super::account::execute(command, session, store, references, deadline).map(Some)
        }
        Command::Group { command } => {
            super::group::execute(command, session, store, references, deadline)
        }
        Command::Contact { command } => {
            super::contact::execute(command, session, store, references, deadline)
        }
        Command::Api {
            command: crate::cli::args::ApiCommand::Call(args),
        } => super::api::execute(args, session, store, references, deadline).map(Some),
        _ => Ok(None),
    }?;
    if let Some(envelope) = &response
        && serde_json::to_vec(envelope)
            .map_err(|_| crate::core::error::AppError::protocol())?
            .len()
            > session.config.response_bytes
    {
        return Err(crate::core::error::AppError::new(
            "RESPONSE_LIMIT",
            "The domain result exceeds its configured output budget.",
            3,
        ));
    }
    Ok(response)
}

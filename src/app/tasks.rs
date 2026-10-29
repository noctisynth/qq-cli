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
    match command {
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
    }
}

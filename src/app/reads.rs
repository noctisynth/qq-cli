//! Read use cases operating on an already authorized session.
use crate::cli::args::{Command, ContactCommand, ConversationCommand, GroupCommand};
use crate::core::{
    domain::Conversation,
    error::{AppError, Result},
    output::Envelope,
};
use crate::messaging::{read, session::Session};
use crate::transport::client::{Action, Backend};
use serde_json::json;
use std::time::{Duration, Instant};

pub fn execute_read<B: Backend>(command: &Command, session: &Session<'_, B>) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(30);
    execute_read_until(command, session, deadline)
}

pub fn execute_read_until<B: Backend>(
    command: &Command,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let (value, scope) = match command {
        Command::Contact {
            command: ContactCommand::Get { user_id },
        } => {
            let target = Conversation::Private(user_id.clone());
            let (values, scope) = session.read(Some(&target), deadline, |backend| {
                read::contacts(backend, deadline)
            })?;
            let value = values
                .into_iter()
                .find(|v| v.id() == user_id)
                .ok_or_else(|| {
                    AppError::new(
                        "CONTACT_NOT_FOUND",
                        "The requested user is not in the returned friend list.",
                        4,
                    )
                })?;
            (value, scope)
        }
        Command::Group {
            command: GroupCommand::Get { group_id },
        } => {
            let target = Conversation::Group(group_id.clone());
            session.read(Some(&target), deadline, |backend| {
                let value = read::group(&backend.call(
                    Action::GroupInfo,
                    &json!({"group_id":group_id}),
                    deadline,
                )?)?;
                if value.id() != group_id {
                    return Err(AppError::protocol());
                }
                Ok(value)
            })?
        }
        Command::Conversation {
            command: ConversationCommand::Get { conversation },
        } => {
            let (values, scope) = session.read(Some(conversation), deadline, |backend| {
                read::conversations(backend, session.config.scan_limit, deadline)
            })?;
            let value = values
                .into_iter()
                .find(|v| v.conversation() == Some(conversation.to_string().as_str()))
                .ok_or_else(|| {
                    AppError::new(
                        "CONVERSATION_NOT_FOUND",
                        "The conversation is not present in the bounded recent snapshot.",
                        4,
                    )
                })?;
            (value, scope)
        }
        _ => {
            return Err(AppError::new(
                "NOT_IMPLEMENTED",
                "This operation is still under implementation; no domain operation was attempted.",
                6,
            ));
        }
    };
    Ok(Envelope::typed(value).scoped(scope))
}

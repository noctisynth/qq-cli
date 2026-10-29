//! Shared selectors for message action scope inference and write controls.
use super::{
    args::{Command, MessageCommand},
    write::WriteArgs,
};
use crate::core::input::Handle;
pub fn write(command: &MessageCommand) -> Option<&WriteArgs> {
    match command {
        MessageCommand::Forward { write, .. }
        | MessageCommand::Recall { write, .. }
        | MessageCommand::MarkRead { write, .. } => Some(write),
        _ => None,
    }
}
pub fn reference(command: &Command) -> Option<&Handle> {
    match command {
        Command::Message {
            command: MessageCommand::Forward { message_ref, .. },
        } => message_ref.first(),
        Command::Message {
            command:
                MessageCommand::Recall { message_ref, .. }
                | MessageCommand::MarkRead {
                    message_ref: Some(message_ref),
                    ..
                },
        } => Some(message_ref),
        _ => None,
    }
}

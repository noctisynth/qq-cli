//! Message, history, collection and media dispatch after instance/session resolution.
use crate::cli::args::{Cli, Command, MessageCommand};
use crate::core::{
    error::{AppError, Result},
    output::Envelope,
};
use crate::messaging::{collection, content, export, history, message, session::Session, write};
use crate::storage::{references::References, state::Store};
use crate::transport::client::Backend;
use std::time::Instant;
pub(super) fn execute<B: Backend>(
    cli: &Cli,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    if let Command::Media {
        command:
            crate::cli::args::MediaCommand::Download {
                media_ref,
                output,
                format,
                stream,
                ..
            },
    } = &cli.command
    {
        if *stream {
            return crate::messaging::media::stream::download(
                media_ref,
                output,
                *format,
                session,
                references.ok_or_else(AppError::storage)?,
                deadline,
            );
        }
        return crate::messaging::media::download_as(
            media_ref,
            output,
            *format,
            session,
            references.ok_or_else(AppError::storage)?,
            deadline,
        );
    }
    if let (Some(request), Some(references)) = (collection::request(&cli.command), references) {
        return collection::execute_until(request, session, references, deadline);
    }
    let write_request = match &cli.command {
        Command::Message {
            command: MessageCommand::Send(args),
        } => Some((args.conversation.clone(), None, &args.content)),
        Command::Message {
            command: MessageCommand::Reply(args),
        } => {
            let locator = message::load_locator(
                references.ok_or_else(AppError::storage)?,
                &args.message_ref,
            )?;
            Some((locator.conversation.clone(), Some(locator), &args.content))
        }
        _ => None,
    };
    if let Some((conversation, reply, args)) = write_request {
        session.authorize(Some(&conversation), true)?;
        let segments = content::prepare(args, &conversation, deadline)?;
        return write::execute(
            write::Plan {
                conversation,
                reply,
                segments,
                attachments: args
                    .image
                    .iter()
                    .map(|path| write::Attachment {
                        kind: crate::messaging::attachments::Kind::Image,
                        path: path.clone(),
                    })
                    .chain(args.file.iter().map(|path| write::Attachment {
                        kind: crate::messaging::attachments::Kind::File,
                        path: path.clone(),
                    }))
                    .collect(),
            },
            args.request_id.as_deref(),
            args.dry_run,
            session,
            store,
            references,
            deadline,
        );
    }
    if let Some(references) = references {
        if let Command::Message {
            command: MessageCommand::Export(args),
        } = &cli.command
        {
            return export::execute_until(args, session, references, deadline);
        }
        if let Some((args, query, search)) = history::request(&cli.command) {
            return history::query(args, query, search, session, references, deadline)
                .map(|page| page.envelope());
        }
        if let Command::Message {
            command:
                MessageCommand::Get {
                    message_ref,
                    expand,
                },
        } = &cli.command
        {
            let locator = message::load_locator(references, message_ref)?;
            return Ok(Envelope::typed(message::get(
                message_ref,
                *expand,
                session,
                references,
                deadline,
            )?)
            .scoped(locator.scope));
        }
    }
    super::reads::execute_read_until(&cli.command, session, deadline)
}

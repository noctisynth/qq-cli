//! Resolve an explicit instance or authenticated reference before opening a backend.
use crate::cli::args::{Cli, Command, MessageCommand};
use crate::core::{
    error::{AppError, Result},
    output::Envelope,
};
use crate::messaging::{collection, export, history, message};
use crate::storage::{
    credentials::SystemCredentials,
    references::References,
    state::{Instance, Registry, Store, select},
};
use std::time::Instant;
#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod tests;
pub(super) enum Resolution {
    Immediate(Envelope),
    Selected(Selection),
}
pub(super) struct Selection {
    pub instance: Instance,
    pub credentials: SystemCredentials,
    pub references: Option<References>,
}
pub(super) fn is_write(command: &Command) -> bool {
    if let Command::Api {
        command: crate::cli::args::ApiCommand::Call(args),
    } = command
    {
        return args.request_id.is_some()
            || crate::transport::catalog::effect(&args.action)
                != crate::core::contracts::api::Effect::Read;
    }
    matches!(
        command,
        Command::Message {
            command: MessageCommand::Send(_) | MessageCommand::Reply(_)
        }
    )
}

fn has_run_record(store: &Store, instance: &crate::storage::state::Instance) -> Result<bool> {
    #[cfg(windows)]
    {
        crate::napcat::runtime::run_job::has_record(store, &instance.id, &instance.run_id)
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        crate::napcat::runtime::macos::run::has_record(store, &instance.id, &instance.run_id)
    }
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    {
        let _ = (store, instance);
        Ok(false)
    }
}
pub(super) fn resolve(
    cli: &Cli,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Resolution> {
    let infer_scope = collection::has_cursor(&cli.command)
        || matches!(
            &cli.command,
            Command::Media {
                command: crate::cli::args::MediaCommand::Download { .. }
            }
        )
        || history::request(&cli.command).is_some_and(|(args, _, _)| args.page.cursor.is_some())
        || matches!(
            &cli.command,
            Command::Message {
                command: MessageCommand::Get { .. } | MessageCommand::Reply(_)
            }
        )
        || matches!(&cli.command,Command::Message {command:MessageCommand::Export(args)} if args.resume);
    // Resolve a fresh request before creating signing state or accessing unrelated credentials.
    let selected = if infer_scope {
        None
    } else {
        Some(select(registered, cli.instance.as_deref())?)
    };
    let write_command = is_write(&cli.command);
    if let Some(instance) = selected.as_ref()
        && instance.state != "ready"
    {
        if matches!(cli.command, Command::Status)
            && !(matches!(instance.state.as_str(), "starting" | "login_required")
                && has_run_record(store, instance)?)
        {
            return Ok(Resolution::Immediate(Envelope::typed(
                crate::messaging::session::unverified_status(instance)?,
            )));
        }
        if !write_command && !matches!(cli.command, Command::Status) {
            crate::messaging::session::require_api_ready(instance)?;
        }
    }
    crate::core::deadline::check(deadline)?;
    let credentials = SystemCredentials::new(store.root())?;
    let references = if matches!(
        &cli.command,
        Command::Api {
            command: crate::cli::args::ApiCommand::Call(_)
        }
    ) || collection::request(&cli.command).is_some()
        || matches!(
            &cli.command,
            Command::Media {
                command: crate::cli::args::MediaCommand::Download { .. }
            }
        )
        || matches!(&cli.command,Command::Message {command:MessageCommand::Send(args)} if !args.content.image.is_empty() || !args.content.file.is_empty())
        || history::request(&cli.command).is_some()
        || matches!(
            &cli.command,
            Command::Message {
                command: MessageCommand::Get { .. }
                    | MessageCommand::Reply(_)
                    | MessageCommand::Export(_)
            }
        ) {
        Some(if infer_scope {
            References::existing(store, &credentials)?
        } else {
            References::open(store, &credentials, deadline)?
        })
    } else {
        None
    };
    let inferred = references
        .as_ref()
        .map(|references| {
            if let Command::Media {
                command: crate::cli::args::MediaCommand::Download { media_ref, .. },
            } = &cli.command
            {
                return references
                    .get::<crate::messaging::message::MediaLocator>("f1", media_ref)
                    .map(|locator| Some(locator.message.scope.instance_id));
            }
            if let Command::Message {
                command: MessageCommand::Get { message_ref, .. },
            } = &cli.command
            {
                return message::load_locator(references, message_ref)
                    .map(|locator| Some(locator.scope.instance_id));
            }
            if let Command::Message {
                command: MessageCommand::Reply(args),
            } = &cli.command
            {
                return message::load_locator(references, &args.message_ref)
                    .map(|locator| Some(locator.scope.instance_id));
            }
            if history::request(&cli.command).is_some() {
                return history::inferred_instance(&cli.command, references);
            }
            if let Command::Message {
                command: MessageCommand::Export(args),
            } = &cli.command
            {
                return export::inferred_instance(args, references);
            }
            collection::inferred_instance(&cli.command, references)
        })
        .transpose()?
        .flatten();
    if cli
        .instance
        .as_ref()
        .zip(inferred.as_ref())
        .is_some_and(|(explicit, inferred)| &**explicit != inferred)
    {
        return Err(AppError::new(
            "INSTANCE_CONFLICT",
            "Explicit instance conflicts with the continuation reference.",
            2,
        ));
    }
    let instance = match selected {
        Some(instance) => instance,
        None => select(registered, cli.instance.as_deref().or(inferred.as_deref()))?,
    };
    if !write_command && !matches!(cli.command, Command::Status) {
        crate::messaging::session::require_api_ready(&instance)?;
    }
    crate::core::deadline::check(deadline)?;
    Ok(Resolution::Selected(Selection {
        instance,
        credentials,
        references,
    }))
}

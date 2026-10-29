//! Application orchestration; command implementations live in focused use-case modules.
mod account;
mod administration;
mod album;
mod api;
mod assets;
pub mod backend;
mod contact;
mod diagnostics;
mod group;
mod message_ops;
mod messages;
mod profile_image;
mod reads;
pub mod runtime;
mod selection;
mod social;
mod status;
mod tasks;
mod watch;
pub use reads::{execute_read, execute_read_until};
use std::time::{Duration, Instant};
pub use watch::execute_watch;

use crate::cli::args::{Cli, Command, NapcatCommand, SchemaCommand};
use crate::cli::{registry, skill};
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::session::Session;
use crate::storage::config::Settings;
use crate::storage::credentials::Credentials;
use crate::storage::state::Store;

pub fn execute(cli: Cli) -> Envelope {
    match dispatch(cli) {
        Ok(result) => result,
        Err(error) => Envelope::failure(error),
    }
}

fn dispatch(cli: Cli) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(cli.command.timeout_seconds()));
    cli.validate()?;
    if let Some(result) = tasks::offline(&cli.command)? {
        return Ok(result);
    }
    match &cli.command {
        Command::Skill { command } => return skill::execute(command),
        Command::Schema {
            command: SchemaCommand::List,
        } => return Ok(Envelope::typed(registry::list())),
        Command::Schema {
            command: SchemaCommand::Get { command },
        } => {
            return Ok(Envelope::typed(registry::describe(command)?));
        }
        _ => (),
    }
    let settings = Settings::load(cli.config.as_deref())?;
    let store = Store::new(settings.state_dir.clone());
    if let Some(result) = runtime::install(&cli, &store, deadline)? {
        return Ok(result);
    }
    let registered = store.read()?;
    crate::core::deadline::check(deadline)?;
    if let Some(result) = runtime::start(&cli, &settings, &store, &registered, deadline)? {
        return Ok(result);
    }
    if let Some(result) = runtime::restart(&cli, &settings, &store, &registered, deadline)? {
        return Ok(result);
    }
    if let Some(result) = runtime::version::execute(&cli, &settings, &store, &registered, deadline)?
    {
        return Ok(result);
    }
    if let Some(result) = administration::execute(&cli, &settings, &store, &registered, deadline)? {
        return Ok(result);
    }
    let selected = match selection::resolve(&cli, &store, &registered, deadline)? {
        selection::Resolution::Immediate(result) => return Ok(result),
        selection::Resolution::Selected(selected) => selected,
    };
    let selection::Selection {
        instance,
        credentials,
        references,
    } = selected;
    let write_command = selection::is_write(&cli.command);
    let transport = if matches!(cli.command, Command::Status)
        && matches!(instance.state.as_str(), "starting" | "login_required")
    {
        crate::app::backend::Context::discover_pending(
            &store,
            &registered,
            &credentials,
            &instance,
            deadline,
        )
    } else {
        crate::app::backend::Context::load(&store, &registered, &credentials, &instance, deadline)
    };
    let client = transport
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|context| {
            let token = credentials.get(&instance.credential_key)?;
            context.client(&instance, &token, &settings.config)
        });
    let backend = match client {
        Ok(client) => client,
        Err(error) if write_command => crate::app::backend::Client::unavailable(error),
        Err(error)
            if matches!(cli.command, Command::Status)
                && matches!(instance.state.as_str(), "starting" | "login_required")
                && matches!(
                    error.code,
                    "SERVICE_NOT_BOUND" | "SERVICE_DISCOVERY_UNAVAILABLE" | "SERVICE_NOT_READY"
                ) =>
        {
            return status::execute_pending(
                &store,
                &credentials,
                &settings.config,
                &registered,
                &instance,
                deadline,
            );
        }
        Err(error) => return Err(error),
    };
    let installation = registered
        .installations
        .iter()
        .find(|i| i.id == instance.installation_id)
        .ok_or_else(|| {
            AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Instance installation is not registered.",
                4,
            )
        })?;
    if matches!(cli.command, Command::Status) {
        return status::execute(&store, &instance, installation, &backend, deadline, || {
            status::execute_pending(
                &store,
                &credentials,
                &settings.config,
                &registered,
                &instance,
                deadline,
            )
        });
    }
    let session = Session {
        backend: &backend,
        config: &settings.config,
        instance: &instance,
        expected_version: &installation.version,
    };
    if let Some(result) = tasks::execute(
        &cli.command,
        &session,
        &store,
        references.as_ref(),
        deadline,
    )? {
        return Ok(result);
    }
    if matches!(
        cli.command,
        Command::Napcat {
            command: NapcatCommand::Start(_)
        }
    ) {
        return runtime::reuse(&session, &store, true, deadline);
    }
    messages::execute(&cli, &session, &store, references.as_ref(), deadline)
}

//! Process stream entry uses the same selection and owned backend as JSON tasks.
use crate::{
    cli::{
        args::{ApiCommand, Cli, Command},
        stream_output::Output,
    },
    core::error::{AppError, Result},
    messaging::session::Session,
    storage::{config::Settings, credentials::Credentials, state::Store},
};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
pub fn execute(cli: &Cli, output: &mut Output, cancel: &AtomicBool) -> Result<u8> {
    cli.validate()?;
    let Command::Api {
        command: ApiCommand::Stream(args),
    } = &cli.command
    else {
        return Err(AppError::invalid());
    };
    let deadline = Instant::now() + Duration::from_secs(u64::from(args.deadline.timeout));
    if args.dry_run {
        output.api_record(&super::api::call_draft(args)?, deadline, true)?;
        return Ok(0);
    }
    let payload = super::api::parameters(args)?;
    let settings = Settings::load(cli.config.as_deref())?;
    let store = Store::new(settings.state_dir.clone());
    let registry = store.read()?;
    let selected = match super::selection::resolve(cli, &store, &registry, deadline)? {
        super::selection::Resolution::Selected(s) => s,
        super::selection::Resolution::Immediate(_) => return Err(AppError::invalid()),
    };
    let instance = &selected.instance;
    let context =
        super::backend::Context::load(&store, &registry, &selected.credentials, instance, deadline);
    let client = context.as_ref().map_err(Clone::clone).and_then(|c| {
        let token = selected.credentials.get(&instance.credential_key)?;
        c.client(instance, &token, &settings.config)
    });
    let backend = match client {
        Ok(client) => client,
        Err(error) if super::selection::is_write(&cli.command) => {
            super::backend::Client::unavailable(error)
        }
        Err(error) => return Err(error),
    };
    let install = registry
        .installations
        .iter()
        .find(|i| i.id == instance.installation_id)
        .ok_or_else(AppError::storage)?;
    let session = Session {
        backend: &backend,
        config: &settings.config,
        instance,
        expected_version: &install.version,
    };
    super::api::stream_execute(
        args,
        payload,
        &session,
        &store,
        selected.references.as_ref(),
        deadline,
        (output, cancel),
    )
}

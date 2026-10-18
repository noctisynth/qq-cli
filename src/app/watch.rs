//! Event subscription application entry; cancellation and output belong to the caller.
use crate::cli::args::{Cli, Command, EventCommand};
use crate::core::error::{AppError, Result};
use crate::messaging::session::Session;
use crate::storage::{
    config::Settings,
    credentials::{Credentials, SystemCredentials},
    references::References,
    state::{Store, select},
};
use std::time::{Duration, Instant};

/// Streaming entry point; caller owns cancellation and output process lifetime.
pub fn execute_watch(
    cli: &Cli,
    sink: &mut impl crate::messaging::watch::Sink,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<u8> {
    cli.validate()?;
    let Command::Event {
        command:
            EventCommand::Watch {
                conversation,
                limit,
                deadline,
            },
    } = &cli.command
    else {
        return Err(AppError::invalid());
    };
    let deadline = Instant::now() + Duration::from_secs(u64::from(deadline.timeout));
    let settings = Settings::load(cli.config.as_deref())?;
    let store = Store::new(settings.state_dir);
    let registry = store.read()?;
    let instance = select(&registry, cli.instance.as_deref())?;
    crate::messaging::session::require_api_ready(&instance)?;
    if !settings.config.permissions.events || !settings.config.permissions.read {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "The local policy does not permit event observation.",
            4,
        ));
    }
    let endpoint = instance.ws_endpoint.as_deref().ok_or_else(|| {
        AppError::new(
            "EVENTS_UNAVAILABLE",
            "Selected instance has no registered forward event endpoint.",
            4,
        )
    })?;
    crate::transport::websocket::validate_endpoint(endpoint)?;
    let installation = registry
        .installations
        .iter()
        .find(|item| item.id == instance.installation_id)
        .ok_or_else(|| {
            AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Instance installation is not registered.",
                4,
            )
        })?;
    let credentials = SystemCredentials::new(store.root())?;
    let transport =
        crate::app::backend::Context::load(&store, &registry, &credentials, &instance, deadline)?;
    let token = credentials.get(&instance.credential_key)?;
    let backend = transport.client(&instance, &token, &settings.config)?;
    let session = Session {
        backend: &backend,
        config: &settings.config,
        instance: &instance,
        expected_version: &installation.version,
    };
    session.authorize(conversation.as_ref(), false)?;
    let references = References::open(&store, &credentials, deadline)?;
    let mut source = transport.source(endpoint, &token)?;
    crate::messaging::watch::run(
        &session,
        &references,
        &mut source,
        sink,
        crate::messaging::watch::Options {
            conversation: conversation.clone(),
            limit: *limit,
            deadline,
            self_messages: None,
        },
        cancel,
    )
}

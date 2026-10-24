//! CLI composition only; platform preparation and process effects stay in runtime::macos.
use super::*;
use crate::{cli::args::StartArgs, napcat::runtime::macos::start as engine};

pub(super) fn restart(
    settings: &Settings,
    store: &Store,
    selected: &crate::storage::state::Instance,
    deadline: Instant,
) -> Result<Envelope> {
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    let context = engine::Context {
        store,
        references: &references,
        credentials: &credentials,
    };
    let started = crate::napcat::runtime::macos::restart::execute(&context, selected, deadline)?;
    finished(&context, settings, started, deadline)
}

pub(super) fn start(
    cli: &Cli,
    args: &StartArgs,
    settings: &Settings,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Envelope> {
    let release =
        crate::napcat::install::catalog::lookup(args.version.as_deref().unwrap_or("4.18.41"))?;
    if release.version != "v4.18.41" {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "macOS startup requires Framework v4.18.41.",
            4,
        ));
    }
    if !args.new && (cli.instance.is_some() || !registered.instances.is_empty()) {
        let selected = select(registered, cli.instance.as_deref())?;
        let installation = registered
            .installations
            .iter()
            .find(|v| v.id == selected.installation_id)
            .ok_or_else(AppError::storage)?;
        ensure_version(Some(release.version), installation)?;
    }
    let matching = registered
        .installations
        .iter()
        .filter(|v| v.version == release.version)
        .count();
    if matching != 1 {
        let mut error = AppError::new(
            if matching == 0 {
                "INSTALLATION_NOT_FOUND"
            } else {
                "INSTALLATION_AMBIGUOUS"
            },
            "Prepare one fixed macOS Framework installation before starting QQ.",
            4,
        );
        if matching == 0 {
            error.next_action = Some(Box::new(
                crate::core::contracts::action::NextAction::Install {
                    version: release.version.into(),
                },
            ));
        }
        return Err(error);
    }
    drop(store.transaction(deadline)?);
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::open(store, &credentials, deadline)?;
    let context = engine::Context {
        store,
        references: &references,
        credentials: &credentials,
    };
    let started = engine::execute(
        &context,
        engine::Options {
            instance: cli.instance.as_deref(),
            version: Some(release.version),
            request: args.request_id.as_deref(),
            new: args.new,
            verify_compatibility: args.verify_compatibility,
        },
        deadline,
    )?;
    finished(&context, settings, started, deadline)
}
fn finished(
    context: &engine::Context<'_, SystemCredentials>,
    settings: &Settings,
    started: engine::Started,
    deadline: Instant,
) -> Result<Envelope> {
    crate::napcat::runtime::macos::compatibility::record(
        context,
        &settings.config,
        &started.instance,
        deadline,
    )?;
    respond(
        context.store,
        context.credentials,
        &settings.config,
        StartResponse {
            instance: started.instance,
            reused: started.reused,
            request_id: started.request,
        },
        deadline,
    )
}

//! CLI composition only; platform preparation and process effects stay in runtime::macos.
use super::*;
use crate::{cli::args::StartArgs, napcat::runtime::macos::start as engine};

pub(super) fn update(
    settings: &Settings,
    store: &Store,
    selected: &crate::storage::state::Instance,
    installation: &crate::storage::state::Installation,
    verify_compatibility: bool,
    deadline: Instant,
) -> Result<Envelope> {
    let release = crate::napcat::runtime::macos::release::lookup(&installation.version)?;
    if installation.version != release.version || installation.digest != release.archive.sha256 {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "macOS maintenance requires the reviewed target installation bytes; no run was stopped.",
            4,
        ));
    }
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    let context = engine::Context {
        store,
        references: &references,
        credentials: &credentials,
    };
    let started = crate::napcat::runtime::macos::maintenance::update(
        &context,
        selected,
        installation,
        verify_compatibility,
        deadline,
    )?;
    finished(&context, settings, started, deadline)
}

pub(super) fn rollback(
    settings: &Settings,
    store: &Store,
    selected: &crate::storage::state::Instance,
    deadline: Instant,
) -> Result<Envelope> {
    if !crate::napcat::runtime::generation::has_history(store, selected)?
        && !crate::napcat::runtime::version_intent::has_pending(store, selected)?
    {
        return Err(AppError::new(
            "ROLLBACK_UNAVAILABLE",
            "No authenticated previous installation is recorded; no rollback was performed.",
            4,
        ));
    }
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    let context = engine::Context {
        store,
        references: &references,
        credentials: &credentials,
    };
    let started =
        crate::napcat::runtime::macos::maintenance::rollback(&context, selected, deadline)?;
    finished(&context, settings, started, deadline)
}

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
    if let Some(version) = args.version.as_deref() {
        crate::napcat::runtime::macos::release::lookup(version)?;
    }
    let selected = if !args.new && (cli.instance.is_some() || !registered.instances.is_empty()) {
        Some(select(registered, cli.instance.as_deref())?)
    } else {
        None
    };
    let installation = selected
        .as_ref()
        .map(|selected| {
            let found = registered
                .installations
                .iter()
                .filter(|installed| installed.id == selected.installation_id)
                .collect::<Vec<_>>();
            match found.as_slice() {
                [installed] => Ok(*installed),
                _ => Err(AppError::storage()),
            }
        })
        .transpose()?;
    let version = args
        .version
        .as_deref()
        .or_else(|| installation.map(|installed| installed.version.as_str()))
        .unwrap_or(crate::napcat::runtime::macos::release::DEFAULT_VERSION);
    let release = crate::napcat::runtime::macos::release::lookup(version)?;
    if let Some(installation) = installation {
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

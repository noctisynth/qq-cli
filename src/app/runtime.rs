//! Install/start application use cases and CLI checkpoints. The runtime engine returns capabilities only.
#[cfg(windows)]
mod existing;
use crate::cli::args::{Cli, Command, NapcatCommand};
use crate::core::{
    domain::Scope,
    error::{AppError, Result},
    output::Envelope,
};
use crate::messaging::session::{Session, remember_identity, status_data};
use crate::napcat::runtime::start_engine::Started;
use crate::storage::{
    config::Settings,
    credentials::{Credentials, SystemCredentials},
    references::References,
    state::{Registry, Store, select},
};
use crate::transport::client::Backend;
use serde_json::json;
use std::time::Instant;

pub(super) fn install(cli: &Cli, store: &Store, deadline: Instant) -> Result<Option<Envelope>> {
    if let Command::Napcat {
        command: NapcatCommand::Install { version, from },
    } = &cli.command
    {
        let release = crate::napcat::install::catalog::lookup(version)?;
        crate::core::deadline::check(deadline)?;
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::open(store, &credentials, deadline)?;
        return crate::napcat::install::acquire::install(
            &release,
            from.as_deref(),
            store,
            &references,
            deadline,
        )
        .map(Some);
    }
    Ok(None)
}
pub(super) fn start(
    cli: &Cli,
    settings: &Settings,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if let Command::Napcat {
        command: NapcatCommand::Start(args),
    } = &cli.command
    {
        // Explicit creation must never accidentally reuse a running instance.
        let automatic_preparation = cli.instance.is_none()
            && (registered.instances.is_empty()
                || matches!(registered.instances.as_slice(), [instance] if matches!(instance.state.as_str(), "preparing" | "prepared")));
        if args.new || automatic_preparation {
            let version = if !args.new
                && let [instance] = registered.instances.as_slice()
            {
                let installation = registered
                    .installations
                    .iter()
                    .find(|item| item.id == instance.installation_id)
                    .ok_or_else(AppError::storage)?;
                ensure_version(args.version.as_deref(), installation)?;
                installation.version.as_str()
            } else {
                args.version.as_deref().unwrap_or("v4.18.38")
            };
            let release = crate::napcat::install::catalog::lookup(version)?;
            // Reject unreviewed inputs before opening credentials or signing state.
            let _inputs = crate::napcat::runtime::startup::inspect(
                store,
                registered,
                &release,
                &crate::napcat::compatibility::reviewed_witnesses(),
                &super::diagnostics::NativeProbe,
                deadline,
            )?;
            #[cfg(windows)]
            {
                let credentials = SystemCredentials::new(store.root())?;
                let references = References::open(store, &credentials, deadline)?;
                let witnesses = crate::napcat::compatibility::reviewed_witnesses();
                let environment = std::env::vars_os().collect();
                let context = crate::napcat::runtime::start_engine::Context {
                    store,
                    references: &references,
                    credentials: &credentials,
                    release: &release,
                    witnesses: &witnesses,
                    probe: &super::diagnostics::NativeProbe,
                    environment: &environment,
                };
                let started = if args.new {
                    let key = args.request_id.as_deref().ok_or_else(AppError::invalid)?;
                    crate::napcat::runtime::start_engine::explicit(&context, key, deadline)?
                } else {
                    crate::napcat::runtime::start_engine::automatic(&context, deadline)?
                };
                return response(store, &credentials, &settings.config, started, deadline)
                    .map(Some);
            }
            #[cfg(not(windows))]
            return Err(AppError::new(
                "STARTUP_NOT_IMPLEMENTED",
                "New instance startup has not completed its lifecycle verification.",
                4,
            ));
        }
        let instance = select(registered, cli.instance.as_deref())?;
        let installation = registered
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
        ensure_version(args.version.as_deref(), installation)?;
        #[cfg(windows)]
        if matches!(
            instance.state.as_str(),
            "stopped" | "preparing" | "prepared"
        ) {
            return existing::start(store, settings, &instance, installation, deadline).map(Some);
        }
        #[cfg(windows)]
        if instance.state == "login_required" {
            let credentials = SystemCredentials::new(store.root())?;
            return response(
                store,
                &credentials,
                &settings.config,
                crate::napcat::runtime::start_engine::Started {
                    instance,
                    reused: true,
                    request_id: None,
                },
                deadline,
            )
            .map(Some);
        }
    }
    Ok(None)
}
pub(super) fn restart(
    cli: &Cli,
    settings: &Settings,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if !matches!(
        cli.command,
        Command::Napcat {
            command: NapcatCommand::Restart
        }
    ) {
        return Ok(None);
    }
    let instance = select(registered, cli.instance.as_deref())?;
    let installation = registered
        .installations
        .iter()
        .find(|item| item.id == instance.installation_id)
        .ok_or_else(AppError::storage)?;
    #[cfg(windows)]
    return existing::restart(store, settings, &instance, installation, deadline).map(Some);
    #[cfg(not(windows))]
    {
        let _ = (settings, store, instance, installation, deadline);
        Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned instance restart requires Windows.",
            4,
        ))
    }
}
fn ensure_version(
    requested: Option<&str>,
    installation: &crate::storage::state::Installation,
) -> Result<()> {
    if requested.is_some_and(|version| {
        version.strip_prefix('v').unwrap_or(version)
            != installation
                .version
                .strip_prefix('v')
                .unwrap_or(&installation.version)
    }) {
        return Err(AppError::new(
            "INSTANCE_VERSION_CONFLICT",
            "Requested version differs from the selected instance; explicitly update its installation first.",
            4,
        ));
    }
    Ok(())
}
/// A retained prelogin process is a checkpoint, never evidence of an online account.
pub fn response(
    store: &Store,
    credentials: &impl Credentials,
    config: &crate::storage::config::Config,
    started: Started,
    deadline: Instant,
) -> Result<crate::core::output::Envelope> {
    if started.instance.state == "login_required" {
        let mut error = AppError::new(
            "AUTH_REQUIRED",
            "The instance is waiting for login in its QQ window; inspect status after authentication.",
            4,
        ).with_details(serde_json::json!({"instanceId":started.instance.id,"state":"login_required","account":null,"reused":started.reused}));
        error.request_id = started.request_id;
        error.next_action = Some(Box::new(
            serde_json::json!({"command":"status","args":{"instance":started.instance.id}}),
        ));
        return Err(error);
    }
    let registry = store.read()?;
    let current = crate::storage::state::select(&registry, Some(&started.instance.id))?;
    if current != started.instance {
        return Err(started.stale());
    }
    let installation = registry
        .installations
        .iter()
        .find(|i| i.id == current.installation_id)
        .ok_or_else(AppError::storage)?;
    let transport =
        crate::app::backend::Context::load(store, &registry, credentials, &current, deadline)?;
    let token = credentials.get(&current.credential_key)?;
    let backend = transport.client(&current, &token, config)?;
    let session = Session {
        backend: &backend,
        config,
        instance: &current,
        expected_version: &installation.version,
    };
    reuse(&session, store, started.reused, deadline)
}
pub(super) fn reuse<B: Backend>(
    session: &Session<'_, B>,
    store: &Store,
    reused: bool,
    deadline: Instant,
) -> Result<Envelope> {
    let instance = session.instance;

    let info = session.verify(deadline)?;
    remember_identity(store, instance, &info.account, deadline)?;
    let mut data = status_data(&info);
    data["instanceId"] = json!(instance.id);
    data["reused"] = json!(reused);
    Ok(Envelope::success(data).scoped(Scope {
        instance_id: instance.id.clone(),
        account: info.account,
    }))
}

use std::time::{Duration, Instant};

use serde_json::json;

use crate::cli::EventCommand;
use crate::cli::{
    Cli, Command, ContactCommand, ConversationCommand, GroupCommand, MessageCommand, NapcatCommand,
    RequestCommand, SchemaCommand,
};
use crate::client::{Action, Backend, inspect};
use crate::config::Settings;
use crate::credentials::{Credentials, SystemCredentials};
use crate::domain::{Conversation, Scope};
use crate::error::{AppError, Result};
use crate::output::Envelope;
use crate::references::References;
use crate::session::{Session, acknowledge_identity, remember_identity, status_data};
use crate::state::{Store, select};
use crate::{
    collection, content, doctor, export, history, installation, ledger, message, read, registry,
    skill, write,
};

pub fn execute(cli: Cli) -> Envelope {
    match dispatch(cli) {
        Ok(result) => result,
        Err(error) => Envelope::failure(error),
    }
}

/// Streaming entry point; caller owns cancellation and output process lifetime.
pub fn execute_watch(
    cli: &Cli,
    sink: &mut impl crate::watch::Sink,
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
    crate::session::require_api_ready(&instance)?;
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
    crate::websocket::validate_endpoint(endpoint)?;
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
        crate::transport::Context::load(&store, &registry, &credentials, &instance, deadline)?;
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
    crate::watch::run(
        &session,
        &references,
        &mut source,
        sink,
        crate::watch::Options {
            conversation: conversation.clone(),
            limit: *limit,
            deadline,
            self_messages: None,
        },
        cancel,
    )
}

fn dispatch(cli: Cli) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(cli.command.timeout_seconds()));
    cli.validate()?;
    match &cli.command {
        Command::Skill { command } => return skill::execute(command),
        Command::Schema {
            command: SchemaCommand::List,
        } => return Ok(Envelope::success(registry::list())),
        Command::Schema {
            command: SchemaCommand::Get { command },
        } => {
            return Ok(Envelope::success(registry::get(command)?));
        }
        _ => (),
    }
    let settings = Settings::load(cli.config.as_deref())?;
    let store = Store::new(settings.state_dir);
    if let Command::Napcat {
        command: NapcatCommand::Install { version, from },
    } = &cli.command
    {
        let release = crate::catalog::lookup(version)?;
        release.require_authorization()?;
        crate::deadline::check(deadline)?;
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::open(&store, &credentials, deadline)?;
        return crate::acquire::install(&release, from.as_deref(), &store, &references, deadline);
    }
    let registered = store.read()?;
    crate::deadline::check(deadline)?;
    if let Command::Napcat {
        command: NapcatCommand::Start(args),
    } = &cli.command
    {
        // Explicit creation must never accidentally reuse a running instance.
        if args.new || registered.instances.is_empty() && cli.instance.is_none() {
            let release = crate::catalog::lookup(args.version.as_deref().unwrap_or("v4.18.38"))?;
            release.require_authorization()?;
            // This read-only pass does not dispatch, generate configuration, or
            // replace the final locked file/content checks required at launch.
            let _inputs = crate::startup::inspect(
                &store,
                &registered,
                &release,
                &crate::compatibility::reviewed_witnesses(),
                &doctor::NativeProbe,
                deadline,
            )?;
            return Err(AppError::new(
                "STARTUP_NOT_IMPLEMENTED",
                "New instance startup has not completed its lifecycle verification.",
                4,
            ));
        }
        let instance = select(&registered, cli.instance.as_deref())?;
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
        if args.version.as_deref().is_some_and(|version| {
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
    }
    if matches!(
        &cli.command,
        Command::Media {
            command: crate::cli::MediaCommand::Prune
        }
    ) {
        if !store
            .root()
            .join("attachments")
            .try_exists()
            .map_err(|_| AppError::storage())?
            && !store
                .root()
                .join("attachment-removals")
                .try_exists()
                .map_err(|_| AppError::storage())?
        {
            return Ok(Envelope::success(
                json!({"cache":"attachments","removedFiles":0,"removedBytes":0,"removedEntries":0,"preservedEntries":0,"retentionSeconds":crate::attachment_cache::RETENTION_SECONDS}),
            ));
        }
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(&store, &credentials)?;
        return crate::attachment_cache::prune(
            &store,
            &references,
            cli.instance.as_deref(),
            std::time::SystemTime::now(),
            deadline,
        );
    }
    if let Command::Doctor { version } = &cli.command {
        return doctor::execute_until(
            &store,
            &settings.config,
            &registered,
            cli.instance.as_deref(),
            version.as_deref(),
            &doctor::NativeProbe,
            deadline,
        );
    }
    if let Command::Request {
        command: RequestCommand::Get { request_id },
    } = &cli.command
    {
        let record = ledger::get(&store, request_id)?;
        if matches!(record.operation.as_str(), "send" | "reply")
            && record.outcome == ledger::Outcome::Accepted
        {
            crate::receipts::validate(
                record.receipt.as_ref().unwrap_or(&serde_json::Value::Null),
                None,
            )
            .map_err(|error| error.with_scope(record.scope.clone()))?;
        }
        if cli
            .instance
            .as_ref()
            .is_some_and(|id| id != &record.scope.instance_id)
        {
            return Err(AppError::new(
                "INSTANCE_CONFLICT",
                "Explicit instance conflicts with the recorded request.",
                2,
            ));
        }
        let mut data = json!({"requestId":record.request_id,"operation":record.operation,"outcome":record.outcome});
        if let Some(receipt) = record.receipt {
            data["receipt"] = receipt;
        }
        return Ok(Envelope::success(data).scoped(record.scope));
    }
    match &cli.command {
        Command::Napcat {
            command: NapcatCommand::Installations,
        } => {
            let values = registered.installations.iter().map(|i| json!({
                "installationId":i.id,"version":i.version,
                "references": registered.instances.iter().filter(|instance| instance.installation_id == i.id).count(),
            })).collect::<Vec<_>>();
            return Ok(Envelope::success(json!(values)));
        }
        Command::Napcat {
            command: NapcatCommand::Instances,
        } => {
            let values = registered.instances.iter().map(|i| json!({
                "instanceId":i.id,"recordedState":i.state,"installationId":i.installation_id,
                "identityVerified":false,
            })).collect::<Vec<_>>();
            return Ok(Envelope::success(json!(values)));
        }
        Command::Status if registered.instances.is_empty() && cli.instance.is_none() => {
            return Ok(Envelope::success(
                json!({"state":"not_started","account":null}),
            ));
        }
        _ => (),
    }
    if let Command::Napcat {
        command: NapcatCommand::Uninstall { installation_id },
    } = &cli.command
    {
        if !registered
            .installations
            .iter()
            .any(|item| item.id == *installation_id)
            && !installation::has_pending(&store, installation_id)?
        {
            return Err(AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Installation is not registered and has no removal journal.",
                4,
            ));
        }
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(&store, &credentials)?;
        return installation::uninstall(&store, installation_id, &references, deadline);
    }
    let infer_scope = collection::has_cursor(&cli.command)
        || matches!(
            &cli.command,
            Command::Media {
                command: crate::cli::MediaCommand::Download { .. }
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
        Some(select(&registered, cli.instance.as_deref())?)
    };
    let write_command = matches!(
        &cli.command,
        Command::Message {
            command: MessageCommand::Send(_) | MessageCommand::Reply(_)
        }
    );
    if let Some(instance) = selected.as_ref()
        && instance.state != "ready"
    {
        if matches!(cli.command, Command::Status) {
            return Ok(Envelope::success(crate::session::unverified_status(
                instance,
            )?));
        }
        if !write_command {
            crate::session::require_api_ready(instance)?;
        }
    }
    crate::deadline::check(deadline)?;
    let credentials = SystemCredentials::new(store.root())?;
    let references = if collection::request(&cli.command).is_some()
        || matches!(
            &cli.command,
            Command::Media {
                command: crate::cli::MediaCommand::Download { .. }
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
            References::existing(&store, &credentials)?
        } else {
            References::open(&store, &credentials, deadline)?
        })
    } else {
        None
    };
    let inferred = references
        .as_ref()
        .map(|references| {
            if let Command::Media {
                command: crate::cli::MediaCommand::Download { media_ref, .. },
            } = &cli.command
            {
                return references
                    .get::<crate::message::MediaLocator>("f1", media_ref)
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
        .is_some_and(|(explicit, inferred)| explicit != inferred)
    {
        return Err(AppError::new(
            "INSTANCE_CONFLICT",
            "Explicit instance conflicts with the continuation reference.",
            2,
        ));
    }
    let instance = match selected {
        Some(instance) => instance,
        None => select(&registered, cli.instance.as_deref().or(inferred.as_deref()))?,
    };
    if !write_command {
        crate::session::require_api_ready(&instance)?;
    }
    crate::deadline::check(deadline)?;
    let transport =
        crate::transport::Context::load(&store, &registered, &credentials, &instance, deadline);
    let client = transport
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|context| {
            let token = credentials.get(&instance.credential_key)?;
            context.client(&instance, &token, &settings.config)
        });
    let backend = match client {
        Ok(client) => client,
        Err(error) if write_command => crate::transport::Client::unavailable(error),
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
        let info = inspect(&backend, deadline)?;
        crate::session::validate_version(&info, &installation.version)?;
        let scope = Scope {
            instance_id: instance.id.clone(),
            account: info.account.clone(),
        };
        if instance
            .account
            .as_ref()
            .is_some_and(|account| account != &info.account)
        {
            acknowledge_identity(&store, &instance, &info.account, deadline)?;
            return Ok(Envelope::success(
                json!({"state":"account_changed","online":info.online,"version":info.version}),
            )
            .scoped(scope));
        }
        remember_identity(&store, &instance, &info.account, deadline)?;
        return Ok(Envelope::success(status_data(&info)).scoped(scope));
    }
    let session = Session {
        backend: &backend,
        config: &settings.config,
        instance: &instance,
        expected_version: &installation.version,
    };
    if matches!(
        cli.command,
        Command::Napcat {
            command: NapcatCommand::Start(_)
        }
    ) {
        let info = session.verify(deadline)?;
        remember_identity(&store, &instance, &info.account, deadline)?;
        let mut data = status_data(&info);
        data["instanceId"] = json!(instance.id);
        data["reused"] = json!(true);
        return Ok(Envelope::success(data).scoped(Scope {
            instance_id: instance.id.clone(),
            account: info.account,
        }));
    }
    if let Command::Media {
        command: crate::cli::MediaCommand::Download {
            media_ref, output, ..
        },
    } = &cli.command
    {
        return crate::media::download(
            media_ref,
            output,
            &session,
            references.as_ref().ok_or_else(AppError::storage)?,
            deadline,
        );
    }
    if let (Some(request), Some(references)) =
        (collection::request(&cli.command), references.as_ref())
    {
        return collection::execute_until(request, &session, references, deadline);
    }
    let write_request = match &cli.command {
        Command::Message {
            command: MessageCommand::Send(args),
        } => Some((args.conversation.clone(), None, &args.content)),
        Command::Message {
            command: MessageCommand::Reply(args),
        } => {
            let locator = message::load_locator(
                references.as_ref().ok_or_else(AppError::storage)?,
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
                        kind: crate::attachments::Kind::Image,
                        path: path.clone(),
                    })
                    .chain(args.file.iter().map(|path| write::Attachment {
                        kind: crate::attachments::Kind::File,
                        path: path.clone(),
                    }))
                    .collect(),
            },
            args.request_id.as_deref(),
            args.dry_run,
            &session,
            &store,
            references.as_ref(),
            deadline,
        );
    }
    if let Some(references) = references.as_ref() {
        if let Command::Message {
            command: MessageCommand::Export(args),
        } = &cli.command
        {
            return export::execute_until(args, &session, references, deadline);
        }
        if let Some((args, query, search)) = history::request(&cli.command) {
            return history::query(args, query, search, &session, references, deadline)
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
            return Ok(Envelope::success(message::get(
                message_ref,
                *expand,
                &session,
                references,
                deadline,
            )?)
            .scoped(locator.scope));
        }
    }
    execute_read_until(&cli.command, &session, deadline)
}

pub fn execute_read<B: Backend>(command: &Command, session: &Session<'_, B>) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(30);
    execute_read_until(command, session, deadline)
}

pub fn execute_read_until<B: Backend>(
    command: &Command,
    session: &Session<'_, B>,
    deadline: Instant,
) -> Result<Envelope> {
    crate::deadline::check(deadline)?;
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
                .find(|v| v["userId"] == *user_id)
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
                if value["groupId"] != *group_id {
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
                .find(|v| v["conversation"] == conversation.to_string())
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
    Ok(Envelope::success(value).scoped(scope))
}

//! Management and local-result commands that do not require a messaging session.
use crate::cli::args::{Cli, Command, NapcatCommand, RequestCommand};
use crate::core::{
    error::{AppError, Result},
    output::Envelope,
};
use crate::napcat::{doctor, install::installation};
use crate::storage::{
    config::Settings,
    credentials::SystemCredentials,
    references::References,
    state::{Registry, Store},
};
use serde_json::json;
use std::time::Instant;

pub(super) fn execute(
    cli: &Cli,
    settings: &Settings,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if matches!(
        &cli.command,
        Command::Media {
            command: crate::cli::args::MediaCommand::Prune
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
            return Ok(Some(Envelope::success(
                json!({"cache":"attachments","removedFiles":0,"removedBytes":0,"removedEntries":0,"preservedEntries":0,"retentionSeconds":crate::messaging::attachments::cache::RETENTION_SECONDS}),
            )));
        }
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        return crate::messaging::attachments::cache::prune(
            store,
            &references,
            cli.instance.as_deref(),
            std::time::SystemTime::now(),
            deadline,
        )
        .map(Some);
    }
    if let Command::Doctor { version } = &cli.command {
        return doctor::execute_until(
            store,
            &settings.config,
            registered,
            cli.instance.as_deref(),
            version.as_deref(),
            &super::diagnostics::NativeProbe,
            deadline,
        )
        .map(Some);
    }
    if let Command::Request {
        command: RequestCommand::Get { request_id },
    } = &cli.command
    {
        let credentials = SystemCredentials::new(store.root())?;
        return crate::requests::query::get(
            store,
            request_id,
            cli.instance.as_deref(),
            &credentials,
            deadline,
        )
        .map(Some);
    }
    match &cli.command {
        Command::Napcat {
            command: NapcatCommand::Installations,
        } => {
            let values = registered.installations.iter().map(|i| json!({
                "installationId":i.id,"version":i.version,
                "references": registered.instances.iter().filter(|instance| instance.installation_id == i.id).count(),
            })).collect::<Vec<_>>();
            return Ok(Some(Envelope::success(json!(values))));
        }
        Command::Napcat {
            command: NapcatCommand::Instances,
        } => {
            let values = registered.instances.iter().map(|i| json!({
                "instanceId":i.id,"recordedState":i.state,"installationId":i.installation_id,
                "identityVerified":false,
            })).collect::<Vec<_>>();
            return Ok(Some(Envelope::success(json!(values))));
        }
        Command::Status if registered.instances.is_empty() && cli.instance.is_none() => {
            return Ok(Some(Envelope::success(
                json!({"state":"not_started","account":null,"identityVerified":false}),
            )));
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
            && !installation::has_pending(store, installation_id)?
        {
            return Err(AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Installation is not registered and has no removal journal.",
                4,
            ));
        }
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        return installation::uninstall(store, installation_id, &references, deadline).map(Some);
    }
    Ok(None)
}

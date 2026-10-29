//! Offline API discovery. Execution is added independently of the catalog.
use crate::{
    cli::args::ApiCommand,
    core::{
        contracts::api::Catalog,
        error::{AppError, Result},
        output::Envelope,
    },
    transport::catalog,
};
use sha2::{Digest, Sha256};
mod call;
mod stream;
pub(super) use call::execute;
pub(super) use call::execute_prepared;
pub(super) use call::{Guard, execute_guarded};
pub(super) use call::{draft as call_draft, parameters};
pub(super) use call::{execute_file, execute_files};
pub(super) use stream::execute as stream_execute;

pub fn offline(command: &ApiCommand) -> Result<Option<Envelope>> {
    if let ApiCommand::Call(args) | ApiCommand::Stream(args) = command {
        return if args.dry_run {
            call::draft(args).map(Some)
        } else {
            Ok(None)
        };
    }
    discovery(command).map(Some)
}

fn discovery(command: &ApiCommand) -> Result<Envelope> {
    match command {
        ApiCommand::Call(_) | ApiCommand::Stream(_) => Err(AppError::invalid()),
        ApiCommand::Describe { action } => Ok(Envelope::typed(catalog::describe(action))),
        ApiCommand::List {
            query,
            limit,
            cursor,
        } => {
            let query = query.as_deref().unwrap_or("");
            if query.len() > 1024 {
                return Err(AppError::invalid());
            }
            let fingerprint = format!(
                "{:x}",
                Sha256::digest(format!("{}:{query}", catalog::COMMIT))
            );
            let prefix = format!("a1.{fingerprint}.");
            let offset = match cursor {
                Some(cursor) => cursor
                    .strip_prefix(&prefix)
                    .and_then(|v| v.parse::<usize>().ok())
                    .ok_or_else(|| {
                        AppError::new(
                            "CURSOR_INVALID",
                            "API cursor does not match this catalog and query.",
                            2,
                        )
                    })?,
                None => 0,
            };
            let names: Vec<_> = catalog::ACTIONS
                .iter()
                .filter(|a| a.contains(query))
                .collect();
            if offset > names.len() {
                return Err(AppError::new(
                    "CURSOR_INVALID",
                    "API cursor is outside this catalog.",
                    2,
                ));
            }
            let end = offset.saturating_add(usize::from(*limit)).min(names.len());
            Ok(Envelope::typed(Catalog {
                items: names[offset..end]
                    .iter()
                    .map(|a| catalog::summary(a))
                    .collect(),
                total: names.len(),
                next_cursor: (end < names.len()).then(|| format!("{prefix}{end}")),
                source_version: catalog::VERSION,
                source_commit: catalog::COMMIT,
            }))
        }
    }
}

//! Explicit native cleanup uses the owned API session and shared one-call intent.
use crate::{
    cli::{
        args::{ApiCallArgs, DeadlineArgs},
        write::WriteArgs,
    },
    core::{
        contracts::{
            backend_cache::*,
            backend_tools::EmptyPayload,
            scalar::{LocalId, No, Yes},
        },
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use std::time::Instant;
pub(super) fn offline(write: &WriteArgs) -> Option<Envelope> {
    write.dry_run.then(|| {
        Envelope::typed(Draft {
            operation: Operation::CleanCache,
            cache: Cache::BackendAndQqMedia,
            dry_run: Yes,
            identity_verified: No,
            cleanup_verified: No,
        })
    })
}
pub(super) fn execute<B: Backend>(
    write: &WriteArgs,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    let args = ApiCallArgs {
        action: "clean_cache".parse().map_err(|_| AppError::protocol())?,
        params_file: None,
        stdin: false,
        request_id: write.request_id.clone(),
        dry_run: false,
        deadline: DeadlineArgs {
            timeout: write.deadline.timeout,
        },
    };
    let payload = serde_json::to_value(EmptyPayload {}).map_err(|_| AppError::protocol())?;
    let (result, scope) =
        super::api::execute_prepared(&args, payload, session, store, references, deadline)?
            .into_api_result()?;
    Ok(Envelope::typed(Receipt {
        operation: Operation::CleanCache,
        cache: Cache::BackendAndQqMedia,
        request_id: LocalId::new(
            result
                .request_id
                .as_deref()
                .ok_or_else(AppError::protocol)?,
        )?,
        outcome: result.outcome.ok_or_else(AppError::protocol)?,
        deduplicated: result.deduplicated,
        cleanup_verified: No,
    })
    .scoped(scope))
}

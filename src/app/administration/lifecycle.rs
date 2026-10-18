//! Explicit maintenance resolves one registered target without entering a messaging session.
use crate::{
    cli::args::Cli,
    core::{error::Result, output::Envelope},
    storage::state::{Registry, Store, select},
};
use std::time::Instant;

pub(super) fn stop(
    cli: &Cli,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Envelope> {
    let instance = select(registered, cli.instance.as_deref())?;
    crate::core::deadline::check(deadline)?;
    #[cfg(windows)]
    {
        use crate::storage::{credentials::SystemCredentials, references::References};
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        let stopped =
            crate::napcat::runtime::stop::execute(store, &references, &instance, deadline)?;
        Ok(Envelope::success(serde_json::json!({
            "instanceId":stopped.instance_id,"state":"stopped","reused":stopped.reused
        })))
    }
    #[cfg(not(windows))]
    {
        let _ = (store, instance);
        Err(crate::core::error::AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned-instance maintenance is supported only on Windows.",
            3,
        ))
    }
}

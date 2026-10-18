//! Explicit maintenance resolves one registered target without entering a messaging session.
use crate::{
    cli::args::Cli,
    core::{error::Result, output::Envelope},
    storage::state::{Registry, Store, select},
};
use std::time::Instant;

pub(super) fn remove(
    store: &Store,
    registered: &Registry,
    instance_id: &str,
    deadline: Instant,
) -> Result<Envelope> {
    #[cfg(windows)]
    {
        use crate::storage::{credentials::SystemCredentials, references::References};
        if !registered.instances.iter().any(|i| i.id == instance_id)
            && !crate::napcat::runtime::remove::has_record(store, instance_id)?
        {
            return Err(crate::core::error::AppError::new(
                "INSTANCE_NOT_FOUND",
                "No registered instance or original removal intent matches this ID.",
                4,
            ));
        }
        crate::core::deadline::check(deadline)?;
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        let removed =
            crate::napcat::runtime::remove::execute(store, &references, instance_id, deadline)?;
        Ok(Envelope::success(
            serde_json::json!({"instanceId":removed.instance_id,"state":"removed","reused":removed.reused}),
        ))
    }
    #[cfg(not(windows))]
    {
        let _ = (store, registered, instance_id, deadline);
        Err(crate::core::error::AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned-instance maintenance is supported only on Windows.",
            3,
        ))
    }
}

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

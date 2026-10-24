#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
use crate::core::contracts::instance::{Removed, RemovedState};
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
use crate::core::contracts::{
    instance::{Stopped, StoppedState},
    scalar::LocalId,
};
// Explicit maintenance resolves one registered target without entering a messaging session.
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
    #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
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
        Ok(Envelope::typed(Removed {
            instance_id: LocalId::new(&removed.instance_id)?,
            state: RemovedState::Removed,
            reused: removed.reused,
        }))
    }
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    {
        let _ = (store, registered, instance_id, deadline);
        Err(crate::core::error::AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned-instance removal requires Windows or Apple Silicon macOS.",
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
    #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
    {
        use crate::storage::{credentials::SystemCredentials, references::References};
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        #[cfg(windows)]
        let stopped =
            crate::napcat::runtime::stop::execute(store, &references, &instance, deadline)?;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let stopped =
            crate::napcat::runtime::macos::stop::execute(store, &references, &instance, deadline)?;
        Ok(Envelope::typed(Stopped {
            instance_id: LocalId::new(&stopped.instance_id)?,
            state: StoppedState::Stopped,
            reused: stopped.reused,
        }))
    }
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    {
        let _ = (store, instance);
        Err(crate::core::error::AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned-instance stop requires Windows or Apple Silicon macOS.",
            3,
        ))
    }
}

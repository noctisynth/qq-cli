//! Optional management progress while OneBot is unavailable; never an account confirmation.
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
use crate::core::error::AppError;
use crate::core::{error::Result, output::Envelope};
use crate::storage::{
    config::Config,
    credentials::Credentials,
    state::{Instance, Registry, Store},
};
use std::time::Instant;

pub(crate) fn execute_pending(
    store: &Store,
    credentials: &impl Credentials,
    config: &Config,
    registry: &Registry,
    instance: &Instance,
    deadline: Instant,
) -> Result<Envelope> {
    let data = crate::messaging::session::unverified_status(instance)?;
    #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
    let mut data = data;
    #[cfg(windows)]
    {
        use crate::napcat::{
            control::{self, owned::OwnedControl},
            runtime::{run_job, runtime_store},
        };
        let installation = registry
            .installations
            .iter()
            .find(|i| i.id == instance.installation_id)
            .ok_or_else(changed)?;
        let refs = crate::storage::references::References::existing(store, credentials)?;
        let (capability, material) = {
            let tx = store.transaction(deadline)?;
            let capability =
                match run_job::reopen_control(&tx, &refs, instance, installation, deadline) {
                    Ok(capability) => capability,
                    Err(error) if error.code == "CONTROL_NOT_BOUND" => {
                        return Ok(Envelope::typed(data));
                    }
                    Err(error) => return Err(error),
                };
            let material = runtime_store::recover_control(
                &tx,
                &refs,
                credentials,
                instance,
                installation,
                deadline,
            )?;
            if capability.endpoint() != material.endpoint() {
                return Err(changed());
            }
            (capability, material)
        };
        let service = capability.service();
        let backend = OwnedControl::new(
            service.job(),
            service.owner(),
            capability.endpoint(),
            config,
        )?;
        let observation = control::observe(&backend, material.token(), deadline)?;
        // No lock is held during the two bounded network operations.
        let tx = store.transaction(deadline)?;
        if tx
            .registry
            .instances
            .iter()
            .filter(|i| i.id == instance.id)
            .count()
            != 1
            || !tx.registry.instances.iter().any(|i| i == instance)
            || tx
                .registry
                .installations
                .iter()
                .filter(|i| i.id == installation.id)
                .count()
                != 1
            || !tx.registry.installations.iter().any(|i| i == installation)
        {
            return Err(changed());
        }
        data.login_observation = Some(observation);
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        use crate::napcat::{
            control::{self, owned::OwnedControl},
            runtime::macos::{material, run},
        };
        let installation = registry
            .installations
            .iter()
            .find(|v| v.id == instance.installation_id)
            .ok_or_else(changed)?;
        let refs = crate::storage::references::References::existing(store, credentials)?;
        let service = {
            let tx = store.transaction(deadline)?;
            run::reopen_service(&tx, &refs, instance, installation, deadline)?
        };
        let Some(endpoint) = service.control_endpoint() else {
            return Ok(Envelope::typed(data));
        };
        let material =
            material::recover_running(store, &refs, credentials, instance, installation, deadline)?;
        if endpoint != format!("http://127.0.0.1:{}/", material.bundle().ports().control) {
            return Err(changed());
        }
        let backend = OwnedControl::for_process(service.owner(), endpoint, config)?;
        let observation = control::observe(&backend, material.bundle().control_token(), deadline)?;
        let tx = store.transaction(deadline)?;
        if !tx.registry.instances.contains(instance)
            || !tx.registry.installations.contains(installation)
        {
            return Err(changed());
        }
        tx.verify_lock()?;
        data.login_observation = Some(observation);
    }
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    {
        let _ = (store, credentials, config, registry, deadline);
    }
    crate::core::deadline::check(deadline)?;
    Ok(Envelope::typed(data))
}

#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
fn changed() -> AppError {
    AppError::new(
        "STATUS_INPUT_CHANGED",
        "Instance, installation or control binding changed during observation; no account was confirmed.",
        4,
    )
}

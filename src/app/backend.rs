//! Shared command/event transport selection. Native run records never downgrade to plain HTTP.
//! Missing or invalid run records never authorize an unverified transport.
use crate::core::error::{AppError, Result};
use crate::storage::config::Config;
use crate::storage::credentials::Credentials;
use crate::storage::state::{Instance, Registry, Store};
use crate::transport::client::{Action, Backend};
use serde_json::Value;
use std::time::Instant;

pub struct Context {
    instance: Instance,
    #[cfg(windows)]
    service: Option<crate::napcat::runtime::run_job::ServiceRun>,
}
/// Type erasure keeps domain modules independent of transport/native lifetimes.
pub struct Client<'a> {
    backend: Result<Box<dyn Backend + 'a>>,
}
impl Client<'_> {
    /// Writes can resolve a matching cached receipt before they need any live transport.
    pub fn unavailable(error: AppError) -> Self {
        Self {
            backend: Err(error),
        }
    }
}
impl Backend for Client<'_> {
    fn call(&self, action: Action, payload: &Value, deadline: Instant) -> Result<Value> {
        self.backend
            .as_ref()
            .map_err(Clone::clone)?
            .call(action, payload, deadline)
    }
    fn media_bytes(
        &self,
        action: Action,
        payload: &Value,
        deadline: Instant,
    ) -> Result<crate::transport::media::Bytes> {
        self.backend
            .as_ref()
            .map_err(Clone::clone)?
            .media_bytes(action, payload, deadline)
    }
}
impl Context {
    /// Explicit pending-status recovery. Only this path may create a first signed
    /// service binding; domain reads and doctor retain the read-only load path.
    pub fn discover_pending(
        store: &Store,
        registry: &Registry,
        credentials: &impl Credentials,
        instance: &Instance,
        deadline: Instant,
    ) -> Result<Self> {
        match Self::load(store, registry, credentials, instance, deadline) {
            Err(error) if error.code == "SERVICE_NOT_BOUND" => {
                #[cfg(windows)]
                {
                    let installation = registry
                        .installations
                        .iter()
                        .find(|i| i.id == instance.installation_id)
                        .ok_or_else(mismatch)?;
                    let refs =
                        crate::storage::references::References::existing(store, credentials)?;
                    let tx = store.transaction(deadline)?;
                    let service = crate::napcat::runtime::run_job::discover_registered(
                        &tx,
                        &refs,
                        instance,
                        installation,
                        deadline,
                    )?;
                    Ok(Self {
                        instance: instance.clone(),
                        service: Some(service),
                    })
                }
                #[cfg(not(windows))]
                {
                    Err(error)
                }
            }
            result => result,
        }
    }
    /// Read-only, before the caller loads the API token. No API/socket/process creation.
    pub fn load(
        store: &Store,
        registry: &Registry,
        credentials: &impl Credentials,
        instance: &Instance,
        deadline: Instant,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        #[cfg(windows)]
        {
            let recorded =
                crate::napcat::runtime::run_job::has_record(store, &instance.id, &instance.run_id)?;
            if recorded {
                let installation = registry
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
                let refs = crate::storage::references::References::existing(store, credentials)?;
                let service = crate::napcat::runtime::run_job::reopen_service(
                    store,
                    &refs,
                    instance,
                    installation,
                    deadline,
                )?;
                return Ok(Self {
                    instance: instance.clone(),
                    service: Some(service),
                });
            }
            Err(required())
        }
        #[cfg(not(windows))]
        {
            let _ = (store, registry, credentials, instance);
            Err(required())
        }
    }
    pub fn client<'a>(
        &'a self,
        instance: &Instance,
        token: &str,
        config: &Config,
    ) -> Result<Client<'a>> {
        if instance.id != self.instance.id
            || instance.run_id != self.instance.run_id
            || instance.endpoint != self.instance.endpoint
            || instance.credential_key != self.instance.credential_key
            || instance.installation_id != self.instance.installation_id
        {
            return Err(mismatch());
        }
        #[cfg(windows)]
        if let Some(service) = &self.service {
            return Ok(Client {
                backend: Ok(Box::new(crate::transport::owned_http::OwnedOneBot::new(
                    service.job(),
                    service.owner(),
                    &instance.endpoint,
                    token,
                    config,
                )?)),
            });
        }
        #[cfg(not(windows))]
        let _ = (token, config);
        Err(required())
    }
    pub fn source<'a>(
        &'a self,
        endpoint: &'a str,
        token: &'a str,
    ) -> Result<crate::messaging::watch::SocketSource<'a>> {
        if self.instance.ws_endpoint.as_deref() != Some(endpoint) {
            return Err(mismatch());
        }
        #[cfg(windows)]
        if let Some(service) = &self.service {
            return Ok(crate::messaging::watch::SocketSource::owned(
                endpoint,
                token,
                service.job(),
                service.owner(),
            ));
        }
        #[cfg(not(windows))]
        let _ = token;
        Err(required())
    }
}
fn required() -> AppError {
    AppError::new(
        "RUNTIME_RECORD_REQUIRED",
        "The managed run has no durable service record; no API request was attempted.",
        4,
    )
}
fn mismatch() -> AppError {
    AppError::new(
        "TRANSPORT_CONTEXT_MISMATCH",
        "The selected transport does not match this instance or event endpoint; no connection was attempted.",
        4,
    )
}

#[cfg(test)]
mod tests;

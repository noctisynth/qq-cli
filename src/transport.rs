//! Shared command/event transport selection. Native run records never downgrade to plain HTTP.
//! Missing or invalid run records never authorize an unverified transport.
use crate::client::{Action, Backend};
use crate::config::Config;
use crate::credentials::Credentials;
use crate::error::{AppError, Result};
use crate::state::{Instance, Registry, Store};
use serde_json::Value;
use std::time::Instant;

pub struct Context {
    instance: Instance,
    #[cfg(windows)]
    service: Option<crate::run_job::ServiceRun>,
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
    ) -> Result<crate::media_transport::Bytes> {
        self.backend
            .as_ref()
            .map_err(Clone::clone)?
            .media_bytes(action, payload, deadline)
    }
}
impl Context {
    /// Read-only, before the caller loads the API token. No API/socket/process creation.
    pub fn load(
        store: &Store,
        registry: &Registry,
        credentials: &impl Credentials,
        instance: &Instance,
        deadline: Instant,
    ) -> Result<Self> {
        crate::deadline::check(deadline)?;
        #[cfg(windows)]
        {
            let recorded = crate::run_job::has_record(store, &instance.id, &instance.run_id)?;
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
                let refs = crate::references::References::existing(store, credentials)?;
                let service =
                    crate::run_job::reopen_service(store, &refs, instance, installation, deadline)?;
                return Ok(Self {
                    instance: instance.clone(),
                    service: Some(service),
                });
            }
            Err(required())
        }
        #[cfg(not(windows))]
        {
            let _ = (store, registry, credentials);
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
                backend: Ok(Box::new(crate::owned_http::OwnedOneBot::new(
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
    ) -> Result<crate::watch::SocketSource<'a>> {
        if self.instance.ws_endpoint.as_deref() != Some(endpoint) {
            return Err(mismatch());
        }
        #[cfg(windows)]
        if let Some(service) = &self.service {
            return Ok(crate::watch::SocketSource::owned(
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
mod tests {
    use super::*;
    use std::time::Duration;
    struct NoCredentials;
    impl Credentials for NoCredentials {
        fn get(&self, _: &str) -> Result<String> {
            panic!("no credential access")
        }
        fn set(&self, _: &str, _: &str) -> Result<()> {
            panic!("no credential writes")
        }
        fn delete(&self, _: &str) -> Result<()> {
            panic!("no credential deletion")
        }
    }
    fn instance() -> Instance {
        Instance {
            id: "original-instance".into(),
            run_id: "original-run".into(),
            installation_id: "unused-original".into(),
            endpoint: "http://127.0.0.1:1/".into(),
            ws_endpoint: Some("ws://127.0.0.1:2/event".into()),
            credential_key: "original-synthetic".into(),
            account: None,
            state: "ready".into(),
            process_id: None,
            process_started: None,
        }
    }
    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }
    #[test]
    fn missing_managed_record_refuses_before_credentials_or_state_creation() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let mut instance = instance();
        assert_eq!(
            Context::load(
                &store,
                &Registry::default(),
                &NoCredentials,
                &instance,
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "RUNTIME_RECORD_REQUIRED"
        );
        instance.credential_key =
            crate::runtime_store::api_credential_key(&instance.id, &instance.run_id).unwrap();
        assert_eq!(
            Context::load(
                &store,
                &Registry::default(),
                &NoCredentials,
                &instance,
                deadline()
            )
            .err()
            .unwrap()
            .code,
            "RUNTIME_RECORD_REQUIRED"
        );
        assert!(!store.root().exists());
    }
    #[test]
    fn transport_context_cannot_be_reused_for_another_instance_or_event_endpoint() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::new(temp.path().join("state"));
        let instance = instance();
        let context = Context {
            instance: instance.clone(),
            #[cfg(windows)]
            service: None,
        };
        let mut changed = instance.clone();
        changed.run_id = "other-run".into();
        assert_eq!(
            context
                .client(&changed, "original-token", &Config::default())
                .err()
                .unwrap()
                .code,
            "TRANSPORT_CONTEXT_MISMATCH"
        );
        assert_eq!(
            context
                .source("ws://127.0.0.1:3/event", "original-token")
                .err()
                .unwrap()
                .code,
            "TRANSPORT_CONTEXT_MISMATCH"
        );
        assert!(!store.root().exists());
    }
}

//! Concrete environment and managed-API adapters for the diagnostic contract.
use crate::core::error::Result;
use crate::messaging::session::Session;
use crate::napcat::{
    doctor::Probe,
    qq::{self, Application},
};
use crate::native::platform::{self, Host};
use crate::storage::{
    config::Config,
    credentials::{Credentials, SystemCredentials},
    references::References,
    state::{Installation, Instance, Store},
};
use crate::transport::client::SessionInfo;
use std::path::{Path, PathBuf};
use std::time::Instant;
pub struct NativeProbe;
impl Probe for NativeProbe {
    fn host(&self) -> Host {
        platform::host()
    }
    fn qq_paths(&self) -> Result<Vec<PathBuf>> {
        platform::qq_executables()
    }
    fn inspect_qq(&self, path: &Path, deadline: Instant) -> Result<Application> {
        qq::inspect(path, deadline)
    }
    fn artifact(
        &self,
        store: &Store,
        installation: &Installation,
        deadline: Instant,
    ) -> Result<()> {
        let credentials = SystemCredentials::new(store.root())?;
        let references = References::existing(store, &credentials)?;
        crate::napcat::install::installation::verify(store, installation, &references, deadline)
    }
    fn api(
        &self,
        store: &Store,
        config: &Config,
        instance: &Instance,
        version: &str,
        deadline: Instant,
    ) -> Result<SessionInfo> {
        crate::messaging::session::require_api_ready(instance)?;
        let credentials = SystemCredentials::new(store.root())?;
        let transport = crate::app::backend::Context::load(
            store,
            &store.read()?,
            &credentials,
            instance,
            deadline,
        )?;
        let token = credentials.get(&instance.credential_key)?;
        let backend = transport.client(instance, &token, config)?;
        Session {
            backend: &backend,
            config,
            instance,
            expected_version: version,
        }
        .verify(deadline)
    }
}

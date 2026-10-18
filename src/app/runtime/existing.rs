//! Start a selected stopped/preparing instance without replacing its signing authority.
use super::*;
use crate::storage::state::Instance;
pub(super) fn start(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
    deadline: Instant,
) -> Result<Envelope> {
    let release = crate::napcat::install::catalog::lookup(&installation.version)?;
    crate::napcat::runtime::startup::inspect(
        store,
        &store.read()?,
        &release,
        &crate::napcat::compatibility::reviewed_witnesses(),
        &super::super::diagnostics::NativeProbe,
        deadline,
    )?;
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    let witnesses = crate::napcat::compatibility::reviewed_witnesses();
    let environment = std::env::vars_os().collect();
    let context = crate::napcat::runtime::start_engine::Context {
        store,
        references: &references,
        credentials: &credentials,
        release: &release,
        witnesses: &witnesses,
        probe: &super::super::diagnostics::NativeProbe,
        environment: &environment,
    };
    let started = crate::napcat::runtime::start_engine::selected(&context, instance, deadline)?;
    response(store, &credentials, &settings.config, started, deadline)
}

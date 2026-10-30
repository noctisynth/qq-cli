//! Promote only an exact, still-running original Framework dispatch to local evidence.
use crate::core::error::{AppError, Result};
use crate::napcat::{
    compatibility,
    doctor::Probe,
    install::catalog,
    runtime::{dispatch, launch, run_job, runtime_store, startup},
};
use crate::storage::{
    config::Config,
    credentials::SystemCredentials,
    references::References,
    state::{Instance, Store},
};
use std::time::Instant;

pub(crate) fn record(
    store: &Store,
    config: &Config,
    selected: &Instance,
    probe: &impl Probe,
    deadline: Instant,
) -> Result<()> {
    if selected.state != "ready" {
        return Ok(());
    }
    let registry = store.read()?;
    let installation = registry
        .installations
        .iter()
        .find(|i| i.id == selected.installation_id)
        .ok_or_else(AppError::storage)?;
    let release = catalog::lookup(&installation.version)?;
    let witnesses = compatibility::available_witnesses(store)?;
    let inputs = startup::inspect_trial(store, &registry, &release, &witnesses, probe, deadline)?;
    if compatibility::evaluate(inputs.compatibility(), &witnesses).state
        == compatibility::Verdict::Supported
    {
        return Ok(());
    }
    let online = probe.api(store, config, selected, &installation.version, deadline)?;
    if !online.online || selected.account.as_deref() != Some(online.account.as_str()) {
        return Err(conflict());
    }
    let credentials = SystemCredentials::new(store.root())?;
    let refs = References::existing(store, &credentials)?;
    let tx = store.transaction(deadline)?;
    if !tx.registry.instances.iter().any(|i| i == selected)
        || !tx.registry.installations.iter().any(|i| i == installation)
    {
        return Err(conflict());
    }
    // Fresh immutable inputs under the same transaction; this never permits a launch.
    let fresh = startup::inspect_trial(store, &tx.registry, &release, &witnesses, probe, deadline)?;
    if inputs.compatibility() != fresh.compatibility() {
        return Err(conflict());
    }
    let material =
        runtime_store::recover_control(&tx, &refs, &credentials, selected, installation, deadline)?;
    let mut environment = launch::environment(
        std::env::vars_os(),
        inputs
            .application()
            .executable
            .parent()
            .ok_or_else(conflict)?,
        &material.directory,
    )?;
    let mut bootstrap = launch::bootstrap(
        &inputs.application().executable,
        &installation.directory,
        &material.directory,
    )?;
    if material.bundle.friend_requests_enabled() {
        launch::friend_requests_entry(&mut bootstrap, &material.directory)?;
        environment.insert(
            "QQ_CLI_FRAMEWORK_ROOT".into(),
            installation.directory.as_os_str().into(),
        );
    }
    let mut hashes = vec![dispatch::input_hash(
        inputs.compatibility(),
        &material.plan,
        &bootstrap,
        &environment,
        &material.bundle,
    )?];
    // Legacy v1 runs used the verified installation as CWD. Do not rewrite the old journal.
    bootstrap.current_directory = installation
        .directory
        .canonicalize()
        .map_err(|_| conflict())?;
    hashes.push(dispatch::input_hash(
        inputs.compatibility(),
        &material.plan,
        &bootstrap,
        &environment,
        &material.bundle,
    )?);
    let target = run_job::observed_target(
        store,
        &refs,
        selected,
        installation,
        &inputs.application().executable,
        &hashes,
        deadline,
    )?;
    let after = probe.api(store, config, selected, &installation.version, deadline)?;
    if !after.online || after.account != online.account || after.version != online.version {
        return Err(conflict());
    }
    // Require the original input/process association again immediately before publication.
    run_job::observed_target(
        store,
        &refs,
        selected,
        installation,
        &inputs.application().executable,
        &hashes,
        deadline,
    )?;
    compatibility::local::record(&tx, &refs, inputs.compatibility().clone(), target)
}
fn conflict() -> AppError {
    AppError::new(
        "STARTUP_EVIDENCE_UNVERIFIED",
        "Original launch inputs and live Framework identity could not be verified; no local compatibility evidence was recorded.",
        4,
    )
}

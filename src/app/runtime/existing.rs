//! Start a selected stopped/preparing instance without replacing its signing authority.
use super::*;
use crate::storage::state::Instance;
pub(super) fn guard_start(store: &Store, instance: &Instance, deadline: Instant) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    if !crate::napcat::runtime::version_intent::has_pending(store, instance)? {
        return Ok(());
    }
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    crate::core::deadline::check(deadline)?;
    crate::napcat::runtime::version_intent::guard_start(
        store,
        &references,
        &store.read()?,
        instance,
    )?;
    Err(crate::napcat::runtime::version_intent::conflict())
}
pub(super) fn start(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
    deadline: Instant,
) -> Result<Envelope> {
    execute(
        store,
        settings,
        instance,
        deadline,
        Action::Start(installation),
        false,
    )
}
pub(super) fn restart(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
    deadline: Instant,
) -> Result<Envelope> {
    guard_start(store, instance, deadline)?;
    execute(
        store,
        settings,
        instance,
        deadline,
        Action::Restart(installation),
        false,
    )
}
pub(super) fn update(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
    verify_compatibility: bool,
    deadline: Instant,
) -> Result<Envelope> {
    execute(
        store,
        settings,
        instance,
        deadline,
        Action::Update(installation),
        verify_compatibility,
    )
}
pub(super) fn rollback(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    deadline: Instant,
) -> Result<Envelope> {
    execute(store, settings, instance, deadline, Action::Rollback, false)
}
#[derive(Clone, Copy)]
enum Action<'a> {
    Start(&'a crate::storage::state::Installation),
    Restart(&'a crate::storage::state::Installation),
    Update(&'a crate::storage::state::Installation),
    Rollback,
}
impl<'a> Action<'a> {
    fn installation(self) -> Option<&'a crate::storage::state::Installation> {
        match self {
            Self::Start(i) | Self::Restart(i) | Self::Update(i) => Some(i),
            Self::Rollback => None,
        }
    }
}
fn inspect(
    store: &Store,
    release: &crate::napcat::install::catalog::Release,
    deadline: Instant,
) -> Result<()> {
    crate::napcat::runtime::startup::inspect(
        store,
        &store.read()?,
        release,
        &crate::napcat::compatibility::available_witnesses(store)?,
        &super::super::diagnostics::NativeProbe,
        deadline,
    )?;
    Ok(())
}
fn execute(
    store: &Store,
    settings: &Settings,
    instance: &Instance,
    deadline: Instant,
    action: Action<'_>,
    verify_compatibility: bool,
) -> Result<Envelope> {
    let observe = || {
        crate::napcat::runtime::observation::record(
            store,
            &settings.config,
            instance,
            &super::super::diagnostics::NativeProbe,
            deadline,
        )
    };
    if matches!(action, Action::Restart(_) | Action::Update(_)) {
        observe()?;
    }
    let release = action
        .installation()
        .map(|i| crate::napcat::install::catalog::lookup(&i.version))
        .transpose()?;
    if let Some(release) = &release
        && !verify_compatibility
    {
        inspect(store, release, deadline)?;
    }
    let credentials = SystemCredentials::new(store.root())?;
    let references = References::existing(store, &credentials)?;
    let release = match release {
        Some(release) => release,
        None => {
            let target = crate::napcat::runtime::start_engine::rollback_target(
                store,
                &references,
                instance,
                deadline,
            )?;
            crate::napcat::install::catalog::lookup(&target.version)?
        }
    };
    if matches!(action, Action::Rollback) {
        observe()?;
        inspect(store, &release, deadline)?;
    }
    let mut witnesses = crate::napcat::compatibility::available_witnesses(store)?;
    if verify_compatibility {
        let inputs = crate::napcat::runtime::startup::inspect_trial(
            store,
            &store.read()?,
            &release,
            &witnesses,
            &super::super::diagnostics::NativeProbe,
            deadline,
        )?;
        witnesses.push(crate::napcat::compatibility::Witness {
            key: inputs.compatibility().clone(),
            verdict: crate::napcat::compatibility::Verdict::Unknown,
            kind: crate::napcat::compatibility::EvidenceKind::AuthorizedStartupTrial,
            reference: "EXPLICIT_STARTUP_TRIAL".into(),
        });
    }
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
    let started = match action {
        Action::Start(_) => {
            crate::napcat::runtime::start_engine::selected(&context, instance, deadline)
        }
        Action::Restart(_) => {
            crate::napcat::runtime::start_engine::restart(&context, instance, deadline)
        }
        Action::Update(_) => {
            crate::napcat::runtime::start_engine::update(&context, instance, deadline)
        }
        Action::Rollback => {
            crate::napcat::runtime::start_engine::rollback(&context, instance, deadline)
        }
    }?;
    response(store, &credentials, &settings.config, started, deadline)
}

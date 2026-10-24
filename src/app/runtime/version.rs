//! Version-maintenance selection before any credential access or process effect.
use super::*;

pub(in crate::app) fn execute(
    cli: &Cli,
    settings: &Settings,
    store: &Store,
    registered: &Registry,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    if matches!(
        &cli.command,
        Command::Napcat {
            command: NapcatCommand::Rollback { .. }
        }
    ) {
        let instance = select(registered, cli.instance.as_deref())?;
        #[cfg(windows)]
        return super::existing::rollback(store, settings, &instance, deadline).map(Some);
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return super::macos::rollback(settings, store, &instance, deadline).map(Some);
        #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
        let _ = instance;
        #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
        return Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned rollback requires Windows.",
            4,
        ));
    }
    let Command::Napcat {
        command:
            NapcatCommand::Update {
                version,
                verify_compatibility,
                ..
            },
    } = &cli.command
    else {
        return Ok(None);
    };
    let release = crate::napcat::install::catalog::lookup(version)?;
    let instance = select(registered, cli.instance.as_deref())?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    crate::napcat::runtime::macos::release::lookup(release.version)?;
    let installations = registered
        .installations
        .iter()
        .filter(|item| {
            item.version.trim_start_matches('v') == release.version.trim_start_matches('v')
        })
        .collect::<Vec<_>>();
    let installation = match installations.as_slice() {
        [installation] => *installation,
        [] => {
            let mut error = AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Prepare the fixed target installation before version maintenance; no run was stopped by this call.",
                4,
            );
            error.next_action = Some(Box::new(
                crate::core::contracts::action::NextAction::Install {
                    version: (release.version).to_owned(),
                },
            ));
            return Err(error);
        }
        _ => {
            return Err(AppError::new(
                "INSTALLATION_AMBIGUOUS",
                "Multiple target installations are registered; no version maintenance occurred.",
                4,
            ));
        }
    };
    #[cfg(windows)]
    return super::existing::update(
        store,
        settings,
        &instance,
        installation,
        *verify_compatibility,
        deadline,
    )
    .map(Some);
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return super::macos::update(
        settings,
        store,
        &instance,
        installation,
        *verify_compatibility,
        deadline,
    )
    .map(Some);
    #[cfg(not(any(windows, all(target_os = "macos", target_arch = "aarch64"))))]
    {
        let _ = (
            settings,
            store,
            instance,
            installation,
            deadline,
            verify_compatibility,
        );
        Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned version maintenance requires Windows.",
            4,
        ))
    }
}

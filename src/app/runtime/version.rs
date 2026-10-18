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
            command: NapcatCommand::Rollback
        }
    ) {
        let instance = select(registered, cli.instance.as_deref())?;
        #[cfg(windows)]
        return super::existing::rollback(store, settings, &instance, deadline).map(Some);
        #[cfg(not(windows))]
        return Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned rollback requires Windows.",
            4,
        ));
    }
    let Command::Napcat {
        command: NapcatCommand::Update { version },
    } = &cli.command
    else {
        return Ok(None);
    };
    let release = crate::napcat::install::catalog::lookup(version)?;
    let instance = select(registered, cli.instance.as_deref())?;
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
                json!({"command":"napcat.install","args":{"version":release.version}}),
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
    return super::existing::update(store, settings, &instance, installation, deadline).map(Some);
    #[cfg(not(windows))]
    {
        let _ = (settings, store, instance, installation, deadline);
        Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Owned version maintenance requires Windows.",
            4,
        ))
    }
}

use std::time::Instant;

use serde_json::{Value, json};

use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::storage::config::Config;
use crate::storage::state::{Instance, Store};
use crate::transport::client::{Backend, SessionInfo, inspect};

/// Couples each domain read/write to the observed instance identity.
pub struct Session<'a, B: Backend> {
    pub backend: &'a B,
    pub config: &'a Config,
    pub instance: &'a Instance,
    pub expected_version: &'a str,
}

pub fn require_api_ready(instance: &Instance) -> Result<()> {
    if instance.state == "ready" {
        return Ok(());
    }
    let mut error = AppError::new(
        "INSTANCE_NOT_READY",
        "Selected instance has no verified running API; inspect its state before continuing.",
        4,
    );
    error.next_action = Some(Box::new(
        json!({"command":"status","args":{"instance":instance.id}}),
    ));
    Err(error)
}
pub fn unverified_status(instance: &Instance) -> Result<Value> {
    if !matches!(
        instance.state.as_str(),
        "preparing"
            | "prepared"
            | "starting"
            | "login_required"
            | "stopped"
            | "stopping"
            | "offline"
            | "failed"
            | "unknown"
    ) {
        return Err(AppError::storage());
    }
    Ok(json!({"state":instance.state,"account":null,"identityVerified":false}))
}

impl<B: Backend> Session<'_, B> {
    pub fn verify(&self, deadline: Instant) -> Result<SessionInfo> {
        crate::core::deadline::check(deadline)?;
        require_api_ready(self.instance)?;
        let info = inspect(self.backend, deadline)?;
        crate::core::deadline::check(deadline)?;
        validate_version(&info, self.expected_version)?;
        if self
            .instance
            .account
            .as_ref()
            .is_some_and(|account| account != &info.account)
        {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Instance identity changed; inspect status before continuing.",
                4,
            )
            .with_details(json!({"instanceId":self.instance.id,"observedAccount":info.account})));
        }
        if !info.online {
            return Err(AppError::new(
                "ACCOUNT_OFFLINE",
                "The selected account is offline.",
                4,
            ));
        }
        Ok(info)
    }

    pub fn authorize(&self, target: Option<&Conversation>, write: bool) -> Result<()> {
        let policy = &self.config.permissions;
        if (write && !policy.send) || (!write && !policy.read) {
            return Err(AppError::new(
                "PERMISSION_DENIED",
                "The local policy does not permit this operation.",
                4,
            ));
        }
        if let Some(target) = target
            && !policy.targets.is_empty()
            && !policy.targets.contains(&target.to_string())
        {
            return Err(AppError::new(
                "TARGET_DENIED",
                "The selected conversation is outside the local policy.",
                4,
            ));
        }
        Ok(())
    }

    pub fn read<T>(
        &self,
        target: Option<&Conversation>,
        deadline: Instant,
        operation: impl FnOnce(&B) -> Result<T>,
    ) -> Result<(T, Scope)> {
        self.read_in_scope(target, None, deadline, operation)
    }

    pub fn read_in_scope<T>(
        &self,
        target: Option<&Conversation>,
        expected: Option<&Scope>,
        deadline: Instant,
        operation: impl FnOnce(&B) -> Result<T>,
    ) -> Result<(T, Scope)> {
        self.authorize(target, false)?;
        let before = self.verify(deadline)?;
        if expected.is_some_and(|scope| {
            scope.instance_id != self.instance.id || scope.account != before.account
        }) {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Reference identity does not match this instance; no domain read was attempted.",
                4,
            ));
        }
        crate::core::deadline::check(deadline)?;
        let result = operation(self.backend)?;
        crate::core::deadline::check(deadline)?;
        let after = self.verify(deadline)?;
        if before.account != after.account {
            return Err(AppError::new(
                "ACCOUNT_CHANGED",
                "Instance identity changed during the operation; results were withheld.",
                4,
            ));
        }
        Ok((
            result,
            Scope {
                instance_id: self.instance.id.clone(),
                account: before.account,
            },
        ))
    }
}

pub fn remember_identity(
    store: &Store,
    instance: &Instance,
    account: &str,
    deadline: Instant,
) -> Result<()> {
    persist_identity(store, instance, account, deadline, false)
}

/// Explicit status inspection acknowledges the observed account without migrating references.
pub fn acknowledge_identity(
    store: &Store,
    instance: &Instance,
    account: &str,
    deadline: Instant,
) -> Result<()> {
    persist_identity(store, instance, account, deadline, true)
}

fn persist_identity(
    store: &Store,
    instance: &Instance,
    account: &str,
    deadline: Instant,
    acknowledge: bool,
) -> Result<()> {
    if instance.account.as_deref() == Some(account) {
        return Ok(());
    }
    let mut transaction = store.transaction(deadline)?;
    let current = transaction
        .registry
        .instances
        .iter_mut()
        .find(|i| i.id == instance.id)
        .ok_or_else(|| {
            AppError::new(
                "INSTANCE_CHANGED",
                "Instance registration changed during the operation.",
                4,
            )
        })?;
    if current.run_id != instance.run_id
        || current.account != instance.account
        || !acknowledge && current.account.as_ref().is_some_and(|id| id != account)
    {
        return Err(AppError::new(
            "ACCOUNT_CHANGED",
            "Instance identity changed; no registration was overwritten.",
            4,
        ));
    }
    current.account = Some(account.to_owned());
    transaction.commit()
}

pub fn status_data(info: &SessionInfo) -> Value {
    let mut data = json!({"state": if info.online {"ready"} else {"offline"}, "online":info.online,"version":info.version});
    if let Some(nickname) = &info.nickname {
        data["nickname"] = json!(nickname);
    }
    data
}

/// Only status may confirm a login after the caller has recovered the exact signed
/// service and inspected it over the owned transport. No process is started here.
pub fn confirm_login(
    store: &Store,
    instance: &Instance,
    installation: &crate::storage::state::Installation,
    info: &SessionInfo,
    deadline: Instant,
) -> Result<bool> {
    validate_version(info, &installation.version)?;
    crate::core::domain::decimal_id(&info.account).map_err(|_| AppError::protocol())?;
    if !matches!(instance.state.as_str(), "starting" | "login_required") {
        return Err(AppError::new(
            "INSTANCE_CHANGED",
            "The instance is not awaiting login confirmation.",
            4,
        ));
    }
    let mut transaction = store.transaction(deadline)?;
    if !transaction
        .registry
        .installations
        .iter()
        .any(|item| item == installation)
    {
        return Err(AppError::new(
            "INSTANCE_CHANGED",
            "Installation changed during login confirmation.",
            4,
        ));
    }
    let current = transaction
        .registry
        .instances
        .iter_mut()
        .find(|item| item.id == instance.id)
        .filter(|item| *item == instance)
        .ok_or_else(|| {
            AppError::new(
                "INSTANCE_CHANGED",
                "Instance changed during login confirmation; no state was overwritten.",
                4,
            )
        })?;
    let changed = instance
        .account
        .as_ref()
        .is_some_and(|account| account != &info.account);
    current.account = Some(info.account.clone());
    current.state = if info.online {
        "ready"
    } else {
        "login_required"
    }
    .into();
    crate::core::deadline::check(deadline)?;
    transaction.commit()?;
    Ok(changed)
}

pub(crate) fn validate_version(info: &SessionInfo, expected: &str) -> Result<()> {
    validate_version_value(&info.version, expected)
}
pub(crate) fn validate_version_value(version: &str, expected: &str) -> Result<()> {
    if version.trim_start_matches('v') != expected.trim_start_matches('v') {
        return Err(AppError::new(
            "VERSION_MISMATCH",
            "The running backend does not match the registered installation.",
            3,
        ));
    }
    // These two fixed releases share the reviewed identity/API contract.
    // Catalog presence alone must not implicitly authorize future protocol versions.
    if !matches!(version.trim_start_matches('v'), "4.18.37" | "4.18.38") {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "This backend version has not passed the registered API contract checks.",
            3,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod version_tests {
    use super::validate_version_value;
    #[test]
    fn reviewed_versions_require_exact_registered_match() {
        for version in ["4.18.37", "4.18.38"] {
            validate_version_value(version, &format!("v{version}")).unwrap();
        }
        assert_eq!(
            validate_version_value("4.18.37", "v4.18.38")
                .unwrap_err()
                .code,
            "VERSION_MISMATCH"
        );
        assert_eq!(
            validate_version_value("4.18.39", "v4.18.39")
                .unwrap_err()
                .code,
            "VERSION_UNSUPPORTED"
        );
    }
}

//! Explicit identity observation and acknowledgement; never used by message commands.
mod pending;
use crate::core::{domain::Scope, error::Result, output::Envelope};
use crate::messaging::session::{acknowledge_identity, remember_identity, status_data};
use crate::storage::state::{Installation, Instance, Store};
use crate::transport::client::{Backend, inspect_with_version_guard};
pub(super) use pending::execute_pending;
use serde_json::json;
use std::time::Instant;
pub(super) fn execute<B: Backend>(
    store: &Store,
    instance: &Instance,
    installation: &Installation,
    backend: &B,
    deadline: Instant,
    pending_observation: impl FnOnce() -> Result<Envelope>,
) -> Result<Envelope> {
    let info = match inspect_with_version_guard(backend, deadline, |version| {
        crate::messaging::session::validate_version_value(version, &installation.version)
    }) {
        Err(error)
            if matches!(instance.state.as_str(), "starting" | "login_required")
                && matches!(error.code, "BACKEND_NOT_LISTENING" | "AUTH_REQUIRED") =>
        {
            return pending_observation();
        }
        result => result?,
    };
    let scope = Scope {
        instance_id: instance.id.clone(),
        account: info.account.clone(),
    };
    if matches!(instance.state.as_str(), "starting" | "login_required") {
        let changed = crate::messaging::session::confirm_login(
            store,
            instance,
            installation,
            &info,
            deadline,
        )?;
        let mut data = status_data(&info);
        if changed {
            data["state"] = json!("account_changed");
        }
        return Ok(Envelope::success(data).scoped(scope));
    }
    if instance
        .account
        .as_ref()
        .is_some_and(|account| account != &info.account)
    {
        acknowledge_identity(store, instance, &info.account, deadline)?;
        return Ok(Envelope::success(
            json!({"state":"account_changed","online":info.online,"version":info.version}),
        )
        .scoped(scope));
    }
    remember_identity(store, instance, &info.account, deadline)?;
    Ok(Envelope::success(status_data(&info)).scoped(scope))
}

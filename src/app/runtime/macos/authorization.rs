//! Select exact credential items before the explicitly requested system UI.
use crate::{
    core::error::{AppError, Result},
    napcat::runtime::runtime_store,
    storage::{
        credentials::SystemCredentials,
        state::{Instance, Registry, Store},
    },
};
use std::time::Instant;

pub(super) fn execute(
    store: &Store,
    credentials: &SystemCredentials,
    registered: &Registry,
    selected: Option<&Instance>,
    deadline: Instant,
) -> Result<()> {
    let keys = keys(selected)?;
    let tx = store.transaction(deadline)?;
    if tx.registry != *registered {
        return Err(changed());
    }
    let lock = tx.lock_identity()?;
    drop(tx);
    credentials.authorize_existing(&keys, deadline, || {
        let tx = store.transaction(deadline)?;
        if tx.lock_identity()? != lock || tx.registry != *registered {
            return Err(changed());
        }
        tx.verify_lock()
    })
}

fn keys(selected: Option<&Instance>) -> Result<Vec<String>> {
    let mut keys = vec!["reference-signing-key".into()];
    if let Some(instance) = selected {
        let identity = runtime_store::identity(&instance.id, &instance.run_id)?;
        let api = runtime_store::api_credential_key(&instance.id, &instance.run_id)?;
        if instance.credential_key != api {
            return Err(changed());
        }
        keys.push(format!("runtime-bundle-{identity}"));
        keys.push(api);
    }
    Ok(keys)
}
fn changed() -> AppError {
    AppError::new(
        "START_INPUT_CHANGED",
        "The selected state or credential scope changed during Keychain authorization; no startup was dispatched.",
        4,
    )
}

//! Authenticated reservation only: no runtime configuration, QQ copy or launch.
use crate::napcat::runtime::instance_creation::{self, Admission, Checkpoint, Creation};
use crate::{
    core::error::{AppError, Result},
    napcat::install::{catalog, installation, macos::source::Source},
    storage::{
        references::References,
        state::{Installation, Store, Transaction, reject_links},
    },
};
use std::time::Instant;

pub fn checkpoint(store: &Store, refs: &References, request: &str) -> Result<Option<Checkpoint>> {
    instance_creation::macos_checkpoint(store, refs, request)
}
/// Holds the local endpoint lease in Fresh until the caller drops/resolves it.
/// A valid fixed Source is static preparation evidence, not startup compatibility.
pub fn reserve(
    store: &Store,
    refs: &References,
    source: &Source,
    installation_id: &str,
    request: &str,
    deadline: Instant,
) -> Result<Creation> {
    crate::core::deadline::check(deadline)?;
    crate::core::domain::request_key(request).map_err(|_| invalid())?;
    let lifecycle = crate::napcat::runtime::lifecycle_lock(store, deadline)?;
    reject_links(store.root())?;
    if store.root().canonicalize().map_err(|_| invalid())? != store.root() {
        return Err(invalid());
    }
    let tx = store.transaction(deadline)?;
    let original_lock = tx.lock_identity()?;
    let selected = select(&tx, installation_id)?;
    crate::requests::keys::guard_start(store, request)?;
    tx.verify_lock()?;
    drop(tx);
    installation::verify(store, &selected, refs, deadline)?;
    source.revalidate(deadline)?;
    lifecycle.verify()?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != original_lock || select(&tx, installation_id)? != selected {
        return Err(invalid());
    }
    let release = catalog::lookup("4.18.41")?;
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lifecycle.verify()
    };
    let result = instance_creation::reserve_in(
        tx,
        Admission {
            refs,
            request_id: request,
            release: &release,
            installation: selected.clone(),
            kind: "n2",
            guard: &guard,
        },
        deadline,
    )?;
    guard()?;
    source.revalidate(deadline)?;
    installation::verify(store, &selected, refs, deadline)?;
    let tx = store.transaction(deadline)?;
    if tx.lock_identity()? != original_lock || select(&tx, installation_id)? != selected {
        return Err(invalid());
    }
    tx.verify_lock()?;
    guard()?;
    Ok(result)
}
fn select(tx: &Transaction<'_>, id: &str) -> Result<Installation> {
    let release = catalog::lookup("4.18.41")?;
    let mut found = tx.registry.installations.iter().filter(|v| v.id == id);
    let selected = found.next().ok_or_else(invalid)?;
    if found.next().is_some()
        || selected.version != release.version
        || selected.digest != release.archive.sha256
    {
        return Err(invalid());
    }
    Ok(selected.clone())
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_CREATION_CONFLICT",
        "Fixed creation inputs or original state authority changed; no replacement instance was selected.",
        4,
    )
}

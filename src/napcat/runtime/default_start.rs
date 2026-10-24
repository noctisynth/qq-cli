//! Stable internal creation key for automatic startup. Never selects an account.
//! Selection runs under the outer startup operation lock, then the state lock.
use crate::{
    core::error::{AppError, Result},
    storage::references::References,
    storage::state::{Instance, Store, atomic_json, private_dir, reject_links},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    version: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
pub enum Selection {
    Existing(Instance),
    Create { request_id: String },
}

pub fn select(
    store: &Store,
    refs: &References,
    version: &str,
    deadline: Instant,
) -> Result<Selection> {
    select_inner(store, refs, version, deadline, false)
}
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn select_macos(
    store: &Store,
    refs: &References,
    version: &str,
    deadline: Instant,
) -> Result<Selection> {
    select_inner(store, refs, version, deadline, true)
}
fn select_inner(
    store: &Store,
    refs: &References,
    version: &str,
    deadline: Instant,
    macos: bool,
) -> Result<Selection> {
    let tx = store.transaction(deadline)?;
    let selected = if tx.registry.instances.is_empty() {
        None
    } else {
        Some(crate::storage::state::select(&tx.registry, None)?)
    };
    if let Some(instance) = &selected {
        let installation = tx
            .registry
            .installations
            .iter()
            .find(|i| i.id == instance.installation_id)
            .ok_or_else(AppError::storage)?;
        if installation.version != version {
            return Err(AppError::new(
                "INSTANCE_VERSION_CONFLICT",
                "Selected instance uses another installation version; no automatic creation occurred.",
                4,
            ));
        }
        if !matches!(instance.state.as_str(), "preparing" | "prepared") {
            return Ok(Selection::Existing(instance.clone()));
        }
    }
    let directory = store.root().join("automatic-start");
    let path = directory.join("active.json");
    reject_links(&path)?;
    let record: Record = match crate::storage::config::read_json::<Pointer>(&path, 4096) {
        Ok(pointer) => {
            if pointer.schema_version != 1 {
                return Err(conflict());
            }
            let record: Record = refs.get("d1", &pointer.reference)?;
            if record.schema_version != 1 || record.version != version {
                return Err(conflict());
            }
            crate::core::domain::request_key(&record.request_id).map_err(|_| conflict())?;
            record
        }
        Err(error) if error.code == "FILE_NOT_FOUND" && selected.is_none() => {
            let mut random = [0_u8; 16];
            getrandom::fill(&mut random).map_err(|_| AppError::storage())?;
            let request_id = format!(
                "automatic-{}",
                random
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            let record = Record {
                schema_version: 1,
                request_id,
                version: version.into(),
            };
            private_dir(&directory)?;
            atomic_json(
                &path,
                &Pointer {
                    schema_version: 1,
                    reference: refs.issue("d1", &record)?,
                },
            )?;
            record
        }
        Err(error) if error.code == "FILE_NOT_FOUND" => return Err(conflict()),
        Err(error) => return Err(error),
    };
    if let Some(instance) = selected {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let plan = if macos {
            crate::napcat::runtime::macos::creation::checkpoint(store, refs, &record.request_id)?
                .map(|v| v.plan)
        } else {
            crate::napcat::runtime::instance_creation::reserved_plan(
                store,
                refs,
                &record.request_id,
            )?
        };
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        let plan = {
            let _ = macos;
            crate::napcat::runtime::instance_creation::reserved_plan(
                store,
                refs,
                &record.request_id,
            )?
        };
        let plan = plan.ok_or_else(conflict)?;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        if macos
            && plan.instance_id == instance.id
            && plan.run_id != instance.run_id
            && super::generation::descends_from(store, refs, &instance, &plan.run_id)?
        {
            return Ok(Selection::Existing(instance));
        }
        if plan.instance_id != instance.id
            || plan.run_id != instance.run_id
            || plan.version != version
        {
            return Err(conflict());
        }
    }
    crate::core::deadline::check(deadline)?;
    Ok(Selection::Create {
        request_id: record.request_id,
    })
}
fn conflict() -> AppError {
    AppError::new(
        "AUTOMATIC_START_CONFLICT",
        "Automatic startup has no matching signed creation checkpoint; inspect the existing instance or original creation request.",
        4,
    )
}

/// Retire only a matching default checkpoint. Caller holds lifecycle and state locks.
/// Signed history remains; deleting this pointer never authorizes replay of its old creation key.
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) fn retire(
    store: &Store,
    refs: &References,
    instance: &Instance,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    guard()?;
    let path = store.root().join("automatic-start").join("active.json");
    reject_links(&path)?;
    let pointer: Pointer = match crate::storage::config::read_json(&path, 4096) {
        Ok(pointer) => pointer,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(()),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(conflict());
    }
    let record: Record = refs.get("d1", &pointer.reference)?;
    if record.schema_version != 1 {
        return Err(conflict());
    }
    #[cfg(windows)]
    let plan = super::instance_creation::reserved_plan(store, refs, &record.request_id)?
        .ok_or_else(conflict)?;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let plan = super::macos::creation::checkpoint(store, refs, &record.request_id)?
        .ok_or_else(conflict)?
        .plan;
    if record.version != plan.version {
        return Err(conflict());
    }
    let same_run = plan.run_id == instance.run_id;
    let same_run = same_run
        || (plan.instance_id == instance.id
            && super::generation::descends_from(store, refs, instance, &plan.run_id)?);
    if plan.instance_id == instance.id && same_run {
        guard()?;
        #[cfg(windows)]
        std::fs::remove_file(path).map_err(|_| AppError::storage())?;
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        remove_pointer(&path, &pointer, guard)?;
    }
    guard()
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn remove_pointer(
    path: &std::path::Path,
    expected: &Pointer,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let directory =
        crate::storage::state::durable::Directory::open(path.parent().ok_or_else(conflict)?)?;
    let file = std::fs::File::options()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|_| conflict())?;
    let mut bytes = Vec::new();
    (&file)
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| conflict())?;
    if bytes.len() > 4096 {
        return Err(conflict());
    }
    let current: Pointer = serde_json::from_slice(&bytes).map_err(|_| conflict())?;
    if current.schema_version != expected.schema_version || current.reference != expected.reference
    {
        return Err(conflict());
    }
    guard()?;
    directory.remove_file("active.json", &file)?;
    guard()
}

#[cfg(test)]
mod tests;

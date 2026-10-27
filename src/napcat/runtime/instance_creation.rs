//! Durable, account-free instance reservation before runtime preparation.
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) mod transition;
use crate::napcat::runtime::ports::Lease;
use crate::{
    core::error::{AppError, Result},
    napcat::compatibility::Witness,
    napcat::doctor::Probe,
    napcat::install::catalog::Release,
    napcat::runtime::runtime_store::{self, Plan},
    napcat::runtime::startup::Inputs,
    storage::references::References,
    storage::state::{
        Installation, Instance, Store, Transaction, atomic_json, private_dir, reject_links,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Reserved,
    Registered,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Record {
    schema_version: u8,
    request_id: String,
    installation: Installation,
    plan: Plan,
    phase: Phase,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

pub enum Creation {
    Fresh(Reservation),
    Existing { instance: Instance, plan: Plan },
}
pub struct Evidence<'a, P: Probe> {
    pub release: &'a Release,
    pub witnesses: &'a [Witness],
    pub probe: &'a P,
}
/// Keeps three local ports reserved until the caller is ready to dispatch.
/// Dropping closes only these local sockets, never any QQ process or account.
pub struct Reservation {
    plan: Plan,
    _ports: Lease,
}
impl Reservation {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
}

/// Read a signed creation identity without reserving ports or mutating state.
/// Callers must still compare it with the current registered instance/run.
pub fn reserved_plan(store: &Store, refs: &References, request_id: &str) -> Result<Option<Plan>> {
    Ok(load(store, refs, request_id)?.map(|record| record.plan))
}
pub struct Checkpoint {
    pub plan: Plan,
    pub installation_id: String,
    pub installation_digest: String,
}
pub fn checkpoint(
    store: &Store,
    refs: &References,
    request_id: &str,
) -> Result<Option<Checkpoint>> {
    // Offline query accepts both authenticated creation formats without
    // migrating them or granting either platform a launch permit.
    let pointer: Pointer = match crate::storage::config::read_json(&path(store, request_id)?, 4096)
    {
        Ok(pointer) => pointer,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    let kind = match pointer.reference.split('.').next() {
        Some("n1") => "n1",
        Some("n2") => "n2",
        _ => return Err(conflict()),
    };
    Ok(
        load_kind(store, refs, request_id, kind)?.map(|record| Checkpoint {
            plan: record.plan,
            installation_id: record.installation.id,
            installation_digest: record.installation.digest,
        }),
    )
}
fn load(store: &Store, refs: &References, request_id: &str) -> Result<Option<Record>> {
    load_kind(store, refs, request_id, "n1")
}
fn load_kind(
    store: &Store,
    refs: &References,
    request_id: &str,
    kind: &str,
) -> Result<Option<Record>> {
    let path = path(store, request_id)?;
    let pointer: Pointer = match crate::storage::config::read_json(&path, 4096) {
        Ok(pointer) => pointer,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(conflict());
    }
    let record: Record = refs.get(kind, &pointer.reference)?;
    if record.schema_version != 1 || record.request_id != request_id {
        return Err(conflict());
    }
    record.plan.ports.validate()?;
    for key in [&record.plan.instance_id, &record.plan.run_id] {
        crate::core::domain::request_key(key).map_err(|_| conflict())?;
    }
    crate::core::domain::request_key(&record.installation.id).map_err(|_| conflict())?;
    if record.installation.digest.len() != 64
        || !record
            .installation
            .digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(conflict());
    }
    Ok(Some(record))
}

impl Inputs {
    /// A stable explicit creation key reserves one instance/run. No credentials,
    /// runtime contents or processes are created; all accounts remain unpreset.
    pub fn reserve_instance<P: Probe>(
        &self,
        store: &Store,
        refs: &References,
        request_id: &str,
        evidence: &Evidence<'_, P>,
        deadline: Instant,
    ) -> Result<Creation> {
        let Evidence {
            release,
            witnesses,
            probe,
        } = evidence;
        crate::core::deadline::check(deadline)?;
        crate::core::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
        let tx = store.transaction(deadline)?;
        crate::requests::keys::guard_start(store, request_id)?;
        let installation =
            self.with_revalidated(&tx, release, witnesses, *probe, deadline, |_, fresh| {
                Ok(fresh.installation().clone())
            })?;
        reserve_in(
            tx,
            Admission {
                refs,
                request_id,
                release,
                installation,
                kind: "n1",
                guard: &|| Ok(()),
            },
            deadline,
        )
    }
}

pub(super) struct Admission<'a> {
    pub refs: &'a References,
    pub request_id: &'a str,
    pub release: &'a Release,
    pub installation: Installation,
    pub kind: &'static str,
    pub guard: &'a dyn Fn() -> Result<()>,
}
pub(super) fn reserve_in(
    mut tx: Transaction<'_>,
    admission: Admission<'_>,
    deadline: Instant,
) -> Result<Creation> {
    let Admission {
        refs,
        request_id,
        release,
        installation,
        kind,
        guard,
    } = admission;
    crate::core::deadline::check(deadline)?;
    crate::core::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
    let store = tx.store();
    crate::requests::keys::guard_start(store, request_id)?;
    let parent = store.root().join("instance-creations");
    let path = parent.join(format!("{:x}.json", Sha256::digest(request_id.as_bytes())));
    reject_links(&path)?;
    let save_claim = |tx: &Transaction<'_>, record: &Record| {
        guard()?;
        tx.verify_lock()?;
        save_kind(&path, refs, record, kind)?;
        tx.verify_lock()?;
        guard()
    };
    guard()?;
    tx.verify_lock()?;
    let mut record = match crate::storage::config::read_json::<Pointer>(&path, 4096) {
        Ok(pointer) => {
            if pointer.schema_version != 1 {
                return Err(conflict());
            }
            let record: Record = refs.get(kind, &pointer.reference)?;
            if record.schema_version != 1
                || record.request_id != request_id
                || record.installation != installation
                || record.plan.version != release.version
            {
                return Err(conflict());
            }
            record.plan.ports.validate()?;
            for key in [&record.plan.instance_id, &record.plan.run_id] {
                crate::core::domain::request_key(key).map_err(|_| conflict())?;
            }
            record
        }
        Err(error) if error.code == "FILE_NOT_FOUND" => {
            #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
            if super::generation::has_origin_for_request(store, request_id)? {
                return Err(conflict());
            }
            let ports = Lease::dynamic(deadline)?;
            let plan = Plan {
                instance_id: random_id("instance")?,
                run_id: random_id("run")?,
                version: release.version.into(),
                ports: ports.ports(),
            };
            require_unremoved(store, &plan.instance_id)?;
            if tx
                .registry
                .instances
                .iter()
                .any(|i| i.id == plan.instance_id || i.run_id == plan.run_id)
            {
                return Err(conflict());
            }
            let mut record = Record {
                schema_version: 1,
                request_id: request_id.into(),
                installation,
                plan,
                phase: Phase::Reserved,
            };
            guard()?;
            tx.verify_lock()?;
            private_dir(&parent)?;
            save_claim(&tx, &record)?; // Claim precedes registration and all preparation.
            tx.registry.instances.push(instance(&record)?);
            crate::core::deadline::check(deadline)?;
            guard()?;
            tx.persist()?;
            guard()?;
            record.phase = Phase::Registered;
            save_claim(&tx, &record)?;
            return Ok(Creation::Fresh(Reservation {
                plan: record.plan,
                _ports: ports,
            }));
        }
        Err(error) => return Err(error),
    };
    require_unremoved(store, &record.plan.instance_id)?;
    if tx
        .registry
        .instances
        .iter()
        .filter(|i| i.id == record.plan.instance_id)
        .count()
        > 1
        || tx
            .registry
            .instances
            .iter()
            .any(|i| i.run_id == record.plan.run_id && i.id != record.plan.instance_id)
    {
        return Err(conflict());
    }
    if let Some(current) = tx
        .registry
        .instances
        .iter()
        .find(|i| i.id == record.plan.instance_id)
    {
        let expected = instance(&record)?;
        if current.run_id != expected.run_id
            || current.installation_id != expected.installation_id
            || current.endpoint != expected.endpoint
            || current.ws_endpoint != expected.ws_endpoint
            || current.credential_key != expected.credential_key
        {
            return Err(conflict());
        }
        if record.phase == Phase::Reserved {
            record.phase = Phase::Registered;
            save_claim(&tx, &record)?;
        }
        guard()?;
        tx.verify_lock()?;
        return Ok(Creation::Existing {
            instance: current.clone(),
            plan: record.plan,
        });
    }
    if record.phase == Phase::Registered {
        return Err(AppError::new(
            "INSTANCE_REMOVED",
            "The instance reserved by this creation key is no longer registered; the key cannot create a replacement.",
            4,
        ));
    }
    let ports = Lease::acquire(record.plan.ports, deadline)?;
    tx.registry.instances.push(instance(&record)?);
    crate::core::deadline::check(deadline)?;
    guard()?;
    tx.persist()?;
    guard()?;
    record.phase = Phase::Registered;
    save_claim(&tx, &record)?;
    Ok(Creation::Fresh(Reservation {
        plan: record.plan,
        _ports: ports,
    }))
}

fn instance(record: &Record) -> Result<Instance> {
    Ok(Instance {
        id: record.plan.instance_id.clone(),
        run_id: record.plan.run_id.clone(),
        installation_id: record.installation.id.clone(),
        endpoint: format!("http://127.0.0.1:{}/", record.plan.ports.http),
        ws_endpoint: Some(format!("ws://127.0.0.1:{}/event", record.plan.ports.events)),
        credential_key: runtime_store::api_credential_key(
            &record.plan.instance_id,
            &record.plan.run_id,
        )?,
        account: None,
        state: "preparing".into(),
        process_id: None,
        process_started: None,
    })
}
fn path(store: &Store, request_id: &str) -> Result<std::path::PathBuf> {
    crate::core::domain::request_key(request_id).map_err(|_| AppError::invalid())?;
    let path = store
        .root()
        .join("instance-creations")
        .join(format!("{:x}.json", Sha256::digest(request_id.as_bytes())));
    reject_links(&path)?;
    Ok(path)
}
#[cfg(test)]
fn save(path: &std::path::Path, refs: &References, record: &Record) -> Result<()> {
    save_kind(path, refs, record, "n1")
}
fn save_kind(path: &std::path::Path, refs: &References, record: &Record, kind: &str) -> Result<()> {
    reject_links(path)?;
    atomic_json(
        path,
        &Pointer {
            schema_version: 1,
            reference: refs.issue(kind, record)?,
        },
    )
}
fn random_id(prefix: &str) -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| AppError::storage())?;
    Ok(format!(
        "{prefix}-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}
/// Removal intent is a negative routing fact, never permission to mutate it.
/// Even a Reserved creation cannot revive its ID once removal has begun.
pub(super) fn require_unremoved(store: &Store, id: &str) -> Result<()> {
    #[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
    if super::remove::has_record(store, id)? {
        return Err(AppError::new(
            "INSTANCE_REMOVED",
            "This instance has been removed or has an original removal intent; its creation key cannot register a replacement.",
            4,
        ));
    }
    let _ = (store, id);
    Ok(())
}
fn conflict() -> AppError {
    AppError::new(
        "INSTANCE_CREATION_CONFLICT",
        "Creation key, installation or reserved instance identity does not match; no replacement was created.",
        4,
    )
}

#[cfg(test)]
mod tests;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(super) fn macos_checkpoint(
    store: &Store,
    refs: &References,
    request_id: &str,
) -> Result<Option<Checkpoint>> {
    Ok(
        load_kind(store, refs, request_id, "n2")?.map(|record| Checkpoint {
            plan: record.plan,
            installation_id: record.installation.id,
            installation_digest: record.installation.digest,
        }),
    )
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(super) fn macos_request(
    store: &Store,
    refs: &References,
    instance: &Instance,
) -> Result<String> {
    let directory = store.root().join("instance-creations");
    reject_links(&directory)?;
    let mut found = None;
    for (index, entry) in std::fs::read_dir(&directory)
        .map_err(|_| conflict())?
        .enumerate()
    {
        if index >= 4096 {
            return Err(conflict());
        }
        let path = entry.map_err(|_| conflict())?.path();
        reject_links(&path)?;
        let pointer: Pointer = crate::storage::config::read_json(&path, 4096)?;
        let kind = match pointer.reference.split('.').next() {
            Some("n1") => "n1",
            Some("n2") => "n2",
            _ => return Err(conflict()),
        };
        let record: Record = refs.get(kind, &pointer.reference)?;
        if pointer.schema_version != 1
            || record.schema_version != 1
            || path != self::path(store, &record.request_id)?
        {
            return Err(conflict());
        }
        if kind == "n2"
            && record.plan.instance_id == instance.id
            && record.plan.run_id == instance.run_id
        {
            if found.is_some() {
                return Err(conflict());
            }
            found = Some(record.request_id);
        }
    }
    found.ok_or_else(conflict)
}

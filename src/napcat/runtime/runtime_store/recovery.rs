//! Read-only secret recovery after startup; mutable runtime contents are not preparation inputs.
use super::{
    MARKER, Owned, Pointer, Secrets, api_credential_key, conflict, credential_conflict, identity,
    optional, read,
};
use crate::core::error::Result;
use crate::napcat::runtime::runtime_config::Bundle;
use crate::storage::{
    credentials::Credentials,
    references::References,
    state::{Installation, Instance, Transaction, reject_links},
};
use std::time::Instant;

/// Private management material; never Debug/Serialize, never persisted by recovery.
pub struct ControlMaterial {
    endpoint: String,
    token: String,
    #[cfg(windows)]
    pub(crate) directory: std::path::PathBuf,
    #[cfg(windows)]
    pub(crate) plan: super::Plan,
    #[cfg(windows)]
    pub(crate) bundle: Bundle,
}
impl ControlMaterial {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Caller holds a state transaction. This is not a process capability or launch permit.
pub fn recover_control(
    tx: &Transaction<'_>,
    refs: &References,
    credentials: &impl Credentials,
    instance: &Instance,
    installation: &Installation,
    deadline: Instant,
) -> Result<ControlMaterial> {
    crate::core::deadline::check(deadline)?;
    if tx
        .registry
        .instances
        .iter()
        .filter(|i| i.id == instance.id)
        .count()
        != 1
        || !tx.registry.instances.iter().any(|i| i == instance)
        || tx
            .registry
            .installations
            .iter()
            .filter(|i| i.id == installation.id)
            .count()
            != 1
        || !tx.registry.installations.iter().any(|i| i == installation)
        || instance.installation_id != installation.id
        || !matches!(
            instance.state.as_str(),
            "starting" | "login_required" | "ready"
        )
    {
        return Err(conflict());
    }
    let id = identity(&instance.id, &instance.run_id)?;
    let base = tx.store().root().canonicalize().map_err(|_| conflict())?;
    let directory = base.join("runtimes").join(&id);
    let stage = base.join("runtimes").join(format!("{id}.stage"));
    let journal = base.join("runtime-preparations").join(format!("{id}.json"));
    for path in [&directory, &stage, &journal] {
        reject_links(path)?;
    }
    if stage.try_exists().map_err(|_| conflict())? || !directory.is_dir() {
        return Err(conflict());
    }
    let pointer: Pointer = crate::storage::config::read_json(&journal, 4096)?;
    let owned: Owned = refs.get("r1", &pointer.reference)?;
    let version = crate::napcat::install::catalog::lookup(&installation.version)?.version;
    if pointer.schema_version != 1
        || owned.schema_version != 1
        || !matches!(owned.template_version, 1 | 2)
        || owned.directory != directory
        || owned.installation_id != installation.id
        || owned.installation_digest != installation.digest
        || owned.plan.instance_id != instance.id
        || owned.plan.run_id != instance.run_id
        || owned.plan.version != version
    {
        return Err(conflict());
    }
    owned.plan.ports.validate()?;
    if instance.endpoint != format!("http://127.0.0.1:{}/", owned.plan.ports.http)
        || instance.ws_endpoint.as_deref()
            != Some(format!("ws://127.0.0.1:{}/event", owned.plan.ports.events).as_str())
        || instance.credential_key != api_credential_key(&instance.id, &instance.run_id)?
    {
        return Err(conflict());
    }
    let marker: Pointer =
        serde_json::from_slice(&read(&directory.join(MARKER))?).map_err(|_| conflict())?;
    if marker.schema_version != 1 || marker.reference != pointer.reference {
        return Err(conflict());
    }
    // All immutable authority checks precede private secret access. Never initialize missing keys.
    let raw = optional(credentials, &format!("runtime-bundle-{id}"), deadline)?
        .ok_or_else(credential_conflict)?;
    if raw.len() > 2048 {
        return Err(credential_conflict());
    }
    let secrets: Secrets = serde_json::from_str(&raw).map_err(|_| credential_conflict())?;
    if secrets.schema_version != 1 || secrets.reference != pointer.reference {
        return Err(credential_conflict());
    }
    let bundle = Bundle::restore(
        version,
        owned.plan.ports,
        secrets.api,
        secrets.control,
        secrets.jwt,
    )?
    .with_friend_requests(owned.template_version)?;
    if optional(credentials, &instance.credential_key, deadline)?.as_deref()
        != Some(bundle.api_token())
    {
        return Err(credential_conflict());
    }
    crate::core::deadline::check(deadline)?;
    Ok(ControlMaterial {
        endpoint: format!("http://127.0.0.1:{}/", owned.plan.ports.control),
        token: bundle.control_token().into(),
        #[cfg(windows)]
        directory,
        #[cfg(windows)]
        plan: owned.plan,
        #[cfg(windows)]
        bundle,
    })
}

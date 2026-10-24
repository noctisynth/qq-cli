//! Private p2 checkpoints, independent of Windows Job/p1 records.
use super::super::{deploy, run::Origin};
use super::{
    members::{LIMIT, Member},
    unverifiable,
};
use crate::{
    core::error::Result,
    storage::{
        references::References,
        state::{self, Installation, Instance, Store},
    },
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub schema_version: u8,
    pub instance: Instance,
    pub installation: Installation,
    pub origin: Option<Origin>,
    pub deployment: Option<deploy::Target>,
    pub deployment_receipt: Option<String>,
    pub members: Vec<Member>,
    pub completed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    schema_version: u8,
    reference: String,
}

fn path(store: &Store, instance: &Instance) -> Result<PathBuf> {
    let id = crate::napcat::runtime::runtime_store::identity(&instance.id, &instance.run_id)?;
    let path = store.root().join("macos-stops").join(format!("{id}.json"));
    state::reject_links(&path)?;
    Ok(path)
}
pub(super) fn load(
    store: &Store,
    refs: &References,
    instance: &Instance,
) -> Result<Option<Record>> {
    let pointer: Pointer = match crate::storage::config::read_json(&path(store, instance)?, 4096) {
        Ok(value) => value,
        Err(error) if error.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(error) => return Err(error),
    };
    if pointer.schema_version != 1 {
        return Err(unverifiable());
    }
    let record: Record = refs.get("p2", &pointer.reference)?;
    validate(&record)?;
    if record.instance.id != instance.id || record.instance.run_id != instance.run_id {
        return Err(unverifiable());
    }
    Ok(Some(record))
}
pub(super) fn save(store: &Store, refs: &References, record: &Record) -> Result<()> {
    validate(record)?;
    let path = path(store, &record.instance)?;
    state::private_dir(path.parent().ok_or_else(unverifiable)?)?;
    let reference = refs.issue("p2", record)?;
    state::atomic_json(
        &path,
        &Pointer {
            schema_version: 1,
            reference,
        },
    )
}
fn validate(record: &Record) -> Result<()> {
    if record.schema_version != 1
        || record.instance.installation_id != record.installation.id
        || record.members.len() > LIMIT
        || !matches!(
            record.instance.state.as_str(),
            "preparing" | "prepared" | "starting" | "login_required" | "ready"
        )
    {
        return Err(unverifiable());
    }
    match &record.origin {
        None => {
            if !record.members.is_empty()
                || record.deployment.is_some()
                || record.deployment_receipt.is_some()
                || !matches!(record.instance.state.as_str(), "preparing" | "prepared")
                || record.instance.account.is_some()
                || record.instance.process_id.is_some()
                || record.instance.process_started.is_some()
            {
                return Err(unverifiable());
            }
        }
        Some(origin) => {
            let signal = origin.signal.as_ref().ok_or_else(unverifiable)?;
            let scope = origin.scope.as_ref().ok_or_else(unverifiable)?;
            origin.owner.validate_for_host()?;
            if signal.identity() != &origin.owner
                || record.members.first().is_none_or(|m| &m.target != signal)
                || origin.target.instance_id != record.instance.id
                || origin.target.run_id != record.instance.run_id
                || origin.target.installation_id != record.installation.id
                || origin.target.installation_digest != record.installation.digest
                || origin.owner.image() != scope.app().join("Contents/MacOS/QQ")
            {
                return Err(unverifiable());
            }
            if let Some(target) = &record.deployment
                && (target.instance_id != record.instance.id
                    || target.run_id != record.instance.run_id
                    || target.installation_id != record.installation.id
                    || target.installation_digest != record.installation.digest)
            {
                return Err(unverifiable());
            }
        }
    }
    if record.deployment.is_some() != record.deployment_receipt.is_some()
        || record.deployment_receipt.as_ref().is_some_and(|digest| {
            digest.len() != 64
                || !digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
    {
        return Err(unverifiable());
    }
    let mut seen = std::collections::BTreeSet::new();
    for (index, member) in record.members.iter().enumerate() {
        member.target.identity().validate_for_host()?;
        if !seen.insert(member.target.identity().process_id())
            || (index == 0 && member.parent.is_some())
            || (index != 0 && member.parent.is_none_or(|parent| parent >= index))
            || record
                .origin
                .as_ref()
                .and_then(|o| o.scope.as_ref())
                .is_none_or(|scope| !member.target.identity().image().starts_with(scope.app()))
        {
            return Err(unverifiable());
        }
    }
    Ok(())
}

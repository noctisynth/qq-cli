//! Registry acknowledgement after a durable completion proof; never repeats native stop effects.
use crate::{
    core::error::Result,
    storage::state::{Installation, Instance, Store},
};
use std::time::Instant;
#[cfg(test)]
mod tests;

pub(super) struct Snapshot {
    instance: Instance,
    installation: Installation,
}
impl Snapshot {
    pub(super) fn new(instance: &Instance, installation: &Installation) -> Self {
        let mut instance = instance.clone();
        instance.state = "stopping".into();
        Self {
            instance,
            installation: installation.clone(),
        }
    }
}
/// Caller has persisted the completed proof and still holds the lifecycle lock.
pub(super) fn complete(store: &Store, expected: &Snapshot, deadline: Instant) -> Result<()> {
    let uncertain = || super::unknown(&expected.instance.id);
    let mut tx = store.transaction(deadline).map_err(|_| uncertain())?;
    if tx
        .registry
        .installations
        .iter()
        .filter(|i| i.id == expected.installation.id)
        .count()
        != 1
        || !tx
            .registry
            .installations
            .iter()
            .any(|i| i == &expected.installation)
    {
        return Err(uncertain());
    }
    let instance = tx
        .registry
        .instances
        .iter_mut()
        .find(|i| i.id == expected.instance.id)
        .ok_or_else(uncertain)?;
    if instance != &expected.instance {
        return Err(uncertain());
    }
    instance.state = "stopped".into();
    tx.commit().map_err(|_| uncertain())
}

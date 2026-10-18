//! Reuse the n1 commit protocol for a stopped instance's authenticated next run.
use super::*;
use crate::storage::state::Transaction;

pub(crate) fn matches(current: &Instance, expected: &Instance) -> bool {
    current.id == expected.id
        && current.run_id == expected.run_id
        && current.installation_id == expected.installation_id
        && current.endpoint == expected.endpoint
        && current.ws_endpoint == expected.ws_endpoint
        && current.credential_key == expected.credential_key
}
pub(crate) struct Proposal<'a> {
    pub source: &'a Instance,
    pub installation: &'a Installation,
    pub plan: &'a Plan,
    pub key: &'a str,
}
impl Proposal<'_> {
    fn record(&self) -> Record {
        Record {
            schema_version: 1,
            request_id: self.key.into(),
            installation: self.installation.clone(),
            plan: self.plan.clone(),
            phase: Phase::Reserved,
        }
    }
    fn checked_record(&self, tx: &Transaction<'_>, refs: &References) -> Result<Option<Record>> {
        crate::requests::keys::guard_start(tx.store(), self.key)?;
        let current = tx
            .registry
            .instances
            .iter()
            .find(|i| i.id == self.source.id)
            .ok_or_else(conflict)?;
        let expected = instance(&self.record())?;
        let previous = load(tx.store(), refs, self.key)?;
        if previous.as_ref().is_some_and(|r| {
            r.installation != *self.installation
                || r.plan != *self.plan
                || (current == self.source && r.phase == Phase::Registered)
        }) {
            return Err(conflict());
        }
        if current != self.source && (!matches(current, &expected) || previous.is_none()) {
            return Err(conflict());
        }
        Ok(previous)
    }
    pub(crate) fn check(&self, tx: &Transaction<'_>, refs: &References) -> Result<()> {
        self.checked_record(tx, refs).map(|_| ())
    }
    pub(crate) fn register(
        &self,
        tx: &mut Transaction<'_>,
        refs: &References,
        ports: Option<Lease>,
        deadline: Instant,
    ) -> Result<Creation> {
        crate::core::deadline::check(deadline)?;
        let previous = self.checked_record(tx, refs)?;
        let mut record = self.record();
        let path = path(tx.store(), self.key)?;
        let index = tx
            .registry
            .instances
            .iter()
            .position(|i| i.id == self.source.id)
            .ok_or_else(conflict)?;
        if tx.registry.instances[index] != *self.source {
            if !previous.is_some_and(|r| r.phase == Phase::Registered) {
                crate::core::deadline::check(deadline)?;
                record.phase = Phase::Registered;
                save(&path, refs, &record)?;
            }
            return Ok(Creation::Existing {
                instance: tx.registry.instances[index].clone(),
                plan: self.plan.clone(),
            });
        }
        let ports = ports.ok_or_else(conflict)?;
        if ports.ports() != self.plan.ports {
            return Err(conflict());
        }
        crate::core::deadline::check(deadline)?;
        private_dir(path.parent().ok_or_else(AppError::storage)?)?;
        if previous.is_none() {
            save(&path, refs, &record)?;
        }
        tx.registry.instances[index] = instance(&record)?;
        crate::core::deadline::check(deadline)?;
        tx.persist()?;
        record.phase = Phase::Registered;
        save(&path, refs, &record)?;
        Ok(Creation::Fresh(Reservation {
            plan: self.plan.clone(),
            _ports: ports,
        }))
    }
}

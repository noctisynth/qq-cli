//! Version transitions retain the original stop authority and one bounded rollback target.
use super::*;

impl Inputs {
    /// Caller holds the lifecycle lock; target inputs are revalidated under the state transaction.
    pub fn reserve_version_run<P: crate::napcat::doctor::Probe>(
        &self,
        store: &Store,
        refs: &References,
        source: &Instance,
        evidence: &Evidence<'_, P>,
        deadline: Instant,
    ) -> Result<NextRun> {
        let mut tx = store.transaction(deadline)?;
        let installation = self.with_revalidated(
            &tx,
            evidence.release,
            evidence.witnesses,
            evidence.probe,
            deadline,
            |_, fresh| Ok(fresh.installation().clone()),
        )?;
        reserve_to(
            &mut tx,
            refs,
            source,
            &installation,
            evidence.release.version,
            deadline,
        )
    }
}

/// Read only, without acquiring another state lock. Caller supplies its locked registry snapshot.
pub fn rollback_installation(
    store: &Store,
    refs: &References,
    current: &Instance,
) -> Result<Option<Installation>> {
    Ok(known_origin(store, refs, current)?.and_then(|record| record.rollback_installation))
}
/// The immediate generation's signed maintenance action; ordinary restarts do not inherit it.
pub fn maintenance_operation(
    store: &Store,
    refs: &References,
    current: &Instance,
) -> Result<Option<super::super::version_intent::Operation>> {
    let Some(record) = known_origin(store, refs, current)? else {
        return Ok(None);
    };
    let Some(reference) = &record.maintenance_reference else {
        return Ok(None);
    };
    let original = record
        .source_installation
        .as_ref()
        .unwrap_or(&record.installation);
    Ok(Some(
        super::super::version_intent::validate_transition(
            store,
            refs,
            reference,
            &record.source,
            original,
            &record.installation,
        )?
        .operation(),
    ))
}

/// Called under the installation transaction: protect rollback and unpublished claimed targets.
pub(crate) fn references_installation(
    store: &Store,
    refs: &References,
    current: &Instance,
    installation_id: &str,
) -> Result<bool> {
    if rollback_installation(store, refs, current)?.is_some_and(|old| old.id == installation_id) {
        return Ok(true);
    }
    let Some(pending) = journal::load(
        store,
        refs,
        false,
        &source_key(&current.id, &current.run_id)?,
    )?
    else {
        return Ok(false);
    };
    if pending.source != *current {
        return Err(conflict());
    }
    Ok(pending.installation.id == installation_id)
}

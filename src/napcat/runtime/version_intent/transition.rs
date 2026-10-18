//! Bind the pre-stop authority to generation publication without acquiring another lock.
use super::*;
pub(crate) fn transition_reference(
    tx: &Transaction<'_>,
    refs: &References,
    source: &Instance,
    original: &Installation,
    target: &Installation,
) -> Result<Option<String>> {
    let marker = tx.registry.maintenance_intents.get(&source.id);
    let Some(reference) = journal::pointer(tx.store(), &source.id, &source.run_id)? else {
        return if marker.is_some() {
            Err(conflict())
        } else {
            Ok(None)
        };
    };
    let intent = validate_transition(tx.store(), refs, &reference, source, original, target)?;
    if marker.is_some_and(|m| m != &reference)
        || (tx
            .registry
            .instances
            .iter()
            .any(|i| i.id == source.id && i.run_id == source.run_id)
            && marker.is_none())
        || (tx
            .registry
            .instances
            .iter()
            .any(|i| i.id == source.id && i.run_id != source.run_id)
            && marker.is_some())
    {
        return Err(conflict());
    }
    Ok(Some(intent.reference))
}
pub(crate) fn validate_transition(
    store: &Store,
    refs: &References,
    reference: &str,
    source: &Instance,
    original: &Installation,
    target: &Installation,
) -> Result<Intent> {
    let intent = journal::load(store, refs, reference)?;
    if intent.stopped_source() != *source
        || intent.record.source_installation != *original
        || intent.target() != target
    {
        return Err(conflict());
    }
    Ok(intent)
}

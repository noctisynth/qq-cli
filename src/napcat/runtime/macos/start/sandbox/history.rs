//! Derive read-only upload namespaces from the authenticated, stopped ancestry.
//! The generation chain is immutable for this run, so new sends or pruning do
//! not change its launch profile. No attachment inventory or new journal exists.
use super::super::*;
use crate::napcat::runtime::{generation, macos::stop, runtime_store};
use std::collections::BTreeSet;

pub(in crate::napcat::runtime::macos::start) fn predecessors(
    store: &Store,
    references: &References,
    runtime: &runtime_store::Plan,
    deadline: Instant,
) -> Result<Vec<String>> {
    let mut current = crate::storage::state::select(&store.read()?, Some(&runtime.instance_id))?;
    if current.run_id != runtime.run_id {
        return Err(conflict());
    }
    let mut seen = BTreeSet::from([current.run_id.clone()]);
    let mut owned = BTreeSet::new();
    loop {
        crate::core::deadline::check(deadline)?;
        let Some((source, installation)) =
            generation::macos_predecessor(store, references, &current)?
        else {
            return Ok(owned.into_iter().collect());
        };
        if source.id != runtime.instance_id || !seen.insert(source.run_id.clone()) {
            return Err(conflict());
        }
        stop::require_completed(store, references, &source, &installation)?;
        owned.insert(runtime_store::identity(&source.id, &source.run_id)?);
        current = source;
    }
}

#[cfg(test)]
mod tests;

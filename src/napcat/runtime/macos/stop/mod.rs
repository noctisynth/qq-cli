//! Original-generation stop. Durable intent precedes every signal; observing
//! the complete confined executable set is separate from process ownership.
mod journal;
mod members;
mod session;

use crate::core::error::AppError;
pub(super) use session::locked;
pub use session::{Stopped, execute};
/// Logical maintenance may consume an original completed p2 without reopening
/// any process or requiring that the old application still exists.
pub(crate) fn require_completed(
    store: &crate::storage::state::Store,
    refs: &crate::storage::references::References,
    instance: &crate::storage::state::Instance,
    installation: &crate::storage::state::Installation,
) -> crate::core::error::Result<()> {
    let record = journal::load(store, refs, instance)?;
    if instance.state != "stopped"
        || !record.is_some_and(|record| {
            let mut expected = record.instance;
            expected.state = "stopped".into();
            record.completed && expected == *instance && record.installation == *installation
        })
    {
        return Err(AppError::new(
            "STOP_PROOF_REQUIRED",
            "Lifecycle maintenance requires the original completed stop proof; no replacement target was selected.",
            4,
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests;

fn unverifiable() -> AppError {
    AppError::new(
        "STOP_UNVERIFIABLE",
        "The original confined process tree could not be verified; unrelated processes were preserved.",
        4,
    )
}
fn unknown() -> AppError {
    let mut error = AppError::new(
        "STOP_OUTCOME_UNKNOWN",
        "Stop is unresolved; inspect this same instance and retain its original stop record.",
        5,
    );
    error.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
    error
}

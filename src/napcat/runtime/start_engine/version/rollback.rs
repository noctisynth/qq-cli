//! Resolve rollback using the platform-independent authenticated intent protocol.
use super::*;
pub use crate::napcat::runtime::version_intent::rollback_target as target;
use crate::napcat::runtime::version_intent::{self, Operation};

pub fn execute<C: Credentials, P: Probe>(
    context: &Context<'_, C, P>,
    selected: &Instance,
    deadline: Instant,
) -> Result<Started> {
    let _operation = operation_lock(context.store, deadline)?;
    let target = target(context.store, context.references, selected, deadline)?;
    if target.version.trim_start_matches('v') != context.release.version.trim_start_matches('v') {
        return Err(version_intent::conflict());
    }
    maintain_locked(context, selected, Operation::Rollback, deadline)
}

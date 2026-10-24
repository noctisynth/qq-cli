//! Scoped official source access; release readers before the owned mount is retired.
use super::*;
use crate::napcat::install::macos::source::{
    Source,
    image::{acquire, attach, journal, release},
};
use std::time::Duration;

fn work_deadline(deadline: Instant) -> Result<Instant> {
    crate::core::deadline::check(deadline)?;
    // Reserve cleanup time inside the public total budget, capped at a minute.
    let remaining = deadline.saturating_duration_since(Instant::now());
    Ok(deadline - (remaining / 4).min(Duration::from_secs(60)))
}

fn completed<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(error)) | (Err(error), Ok(())) => Err(error),
        (Err(mut original), Err(_cleanup)) => {
            original.message = "Preparation failed and source cleanup is unresolved; preserve the original instance and its source records.";
            Err(original)
        }
    }
}

pub(super) fn retire(
    context: &Context<'_, impl Credentials>,
    request: &str,
    deadline: Instant,
) -> Result<()> {
    let work = crate::storage::state::durable::Lock::acquire(
        context.store.root(),
        "qq-source.lock",
        deadline,
        "QQ_SOURCE_BUSY",
    )?;
    let prefix = format!("{:x}", Sha256::digest(request.as_bytes()));
    for attempt in 0..16 {
        work.verify()?;
        let key = format!("source-{prefix}-{attempt}");
        if !journal::needs_retirement(context.store, context.references, &key)? {
            continue;
        }
        let image = acquire::cached(context.store, deadline)?;
        let status = release::release(
            context.store,
            context.references,
            &key,
            &format!("release-{prefix}-{attempt}"),
            image,
            deadline,
        )?;
        if status != journal::release::Status::Released {
            return Err(unresolved());
        }
        journal::cleanup::cleanup(
            context.store,
            context.references,
            &key,
            acquire::cached(context.store, deadline)?,
            deadline,
        )?;
    }
    work.verify()?;
    Ok(())
}

pub(super) fn with_source<T>(
    context: &Context<'_, impl Credentials>,
    request: &str,
    deadline: Instant,
    operation: impl FnOnce(&Source, Instant) -> Result<T>,
) -> Result<T> {
    let work = crate::storage::state::durable::Lock::acquire(
        context.store.root(),
        "qq-source.lock",
        deadline,
        "QQ_SOURCE_BUSY",
    )?;
    let working = work_deadline(deadline)?;
    let prefix = format!("{:x}", Sha256::digest(request.as_bytes()));
    // Completed source mounts are tombstones. A later preparation attempt may
    // need another source read, but may never replay an unknown attach/release.
    for attempt in 0..16 {
        work.verify()?;
        let key = format!("source-{prefix}-{attempt}");
        let image = acquire::acquire(context.store, None, working)?;
        let ready = match attach::attach(context.store, context.references, &key, image, working)? {
            journal::Status::Ready(ready) => ready,
            journal::Status::Released => {
                let image = acquire::acquire(context.store, None, deadline)?;
                journal::cleanup::cleanup(
                    context.store,
                    context.references,
                    &key,
                    image,
                    deadline,
                )?;
                continue;
            }
            journal::Status::Unknown => return Err(unresolved()),
        };
        let result = (|| {
            let source = Source::inspect(&ready.root().join("QQ.app"), working)?;
            work.verify()?;
            let result = operation(&source, working);
            work.verify()?;
            result
        })();
        let image = (*ready).into_image();
        let cleanup = (|| {
            work.verify()?;
            let released = release::release(
                context.store,
                context.references,
                &key,
                &format!("release-{prefix}-{attempt}"),
                image,
                deadline,
            )?;
            if released != journal::release::Status::Released {
                return Err(unresolved());
            }
            let image = acquire::acquire(context.store, None, deadline)?;
            journal::cleanup::cleanup(context.store, context.references, &key, image, deadline)?;
            work.verify()?;
            Ok(())
        })();
        return completed(result, cleanup);
    }
    Err(AppError::new(
        "SOURCE_ATTEMPT_LIMIT",
        "Source preparation attempts reached their limit; inspect the retained history.",
        4,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_has_reserved_budget_and_preserves_the_original_failure() {
        let deadline = Instant::now() + Duration::from_secs(900);
        let working = work_deadline(deadline).unwrap();
        assert_eq!(deadline.duration_since(working), Duration::from_secs(60));
        assert!(working > Instant::now());
        let deadline = Instant::now() + Duration::from_secs(4);
        let working = work_deadline(deadline).unwrap();
        assert!(deadline.duration_since(working) <= Duration::from_secs(1));
        assert!(working > Instant::now());
        assert!(work_deadline(Instant::now() - Duration::from_secs(1)).is_err());
        let mut original = AppError::new("ORIGINAL_PREPARATION", "Original failure", 4);
        original.outcome = Some(crate::core::contracts::write::Outcome::Unknown);
        let cleanup = AppError::new("SOURCE_CLEANUP", "Secondary failure", 5);
        let error = completed::<()>(Err(original), Err(cleanup)).unwrap_err();
        assert_eq!(error.code, "ORIGINAL_PREPARATION");
        assert_eq!(
            error.outcome,
            Some(crate::core::contracts::write::Outcome::Unknown)
        );
        assert!(error.message.contains("source cleanup is unresolved"));
    }
}

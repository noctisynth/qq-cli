//! Scoped official source access; release readers before the owned mount is retired.
use super::*;
use crate::napcat::install::macos::source::{
    Source,
    image::{acquire, attach, journal, release},
};

pub(super) fn with_source<T>(
    context: &Context<'_, impl Credentials>,
    request: &str,
    deadline: Instant,
    operation: impl FnOnce(&Source) -> Result<T>,
) -> Result<T> {
    let prefix = format!("{:x}", Sha256::digest(request.as_bytes()));
    // Completed source mounts are tombstones. A later preparation attempt may
    // need another source read, but may never replay an unknown attach/release.
    for attempt in 0..16 {
        let key = format!("source-{prefix}-{attempt}");
        let image = acquire::acquire(context.store, None, deadline)?;
        let ready = match attach::attach(context.store, context.references, &key, image, deadline)?
        {
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
            let source = Source::inspect(&ready.root().join("QQ.app"), deadline)?;
            operation(&source)
        })();
        let image = (*ready).into_image();
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
        return result;
    }
    Err(AppError::new(
        "SOURCE_ATTEMPT_LIMIT",
        "Source preparation attempts reached their limit; inspect the retained history.",
        4,
    ))
}

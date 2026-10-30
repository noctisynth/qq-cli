//! One bounded history page repairs messages, never notice continuity.
use super::*;
use crate::cli::args::{DeadlineArgs, PageArgs, QueryArgs};
pub(super) fn fill<B: Backend>(
    session: &Session<'_, B>,
    refs: &References,
    watch: &mut Watch<'_>,
    sink: &mut impl Sink,
    options: &Options,
    cancel: &AtomicBool,
) -> Result<()> {
    let Some(target) = &options.conversation else {
        return Ok(());
    };
    if watch.full() || cancel.load(Ordering::Relaxed) {
        return Ok(());
    }
    let deadline = options.operation_deadline();
    watch.renew(deadline);
    let since = chrono::DateTime::from_timestamp(watch.since(), 0)
        .ok_or_else(AppError::invalid)?
        .to_rfc3339();
    let args = QueryArgs {
        conversation: Some(target.clone()),
        since: Some(since.parse().map_err(|_| AppError::invalid())?),
        until: None,
        page: PageArgs {
            limit: watch.remaining().into(),
            cursor: None,
            deadline: DeadlineArgs {
                timeout: 30_u16.into(),
            },
        },
    };
    let before = watch.emitted_count();
    let page = crate::messaging::history::query(&args, None, false, session, refs, deadline)?;
    for message in page.messages {
        if cancel.load(Ordering::Relaxed) || watch.full() {
            break;
        }
        if let Some(event) = watch.recover(message)? {
            record(watch, event, sink, deadline)?;
        }
    }
    watch.recovered_page(&page.info, before);
    Ok(())
}

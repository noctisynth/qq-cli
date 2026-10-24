//! One-shot release intent. Recovery observes disappearance, never replays effects.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Unknown,
    Released,
}

/// All caller-held Source/Ready readers must be dropped before calling this.
/// Only an authenticated completed mount can grant the fresh callback its raw pins.
/// The callback must use guard at each actual unmount/eject effect boundary.
pub fn release(
    store: &Store,
    references: &References,
    mount_request: &str,
    request: &str,
    image: Image,
    deadline: Instant,
    operation: impl FnOnce(mounted::Devices, &dyn Fn() -> Result<()>) -> Result<()>,
) -> Result<Status> {
    crate::core::domain::request_key(mount_request).map_err(|_| conflict())?;
    crate::core::domain::request_key(request).map_err(|_| conflict())?;
    crate::core::deadline::check(deadline)?;
    let lock = Lock::acquire(store.root(), "qq-mount.lock", deadline, "QQ_MOUNT_BUSY")?;
    let input = Input::inspect(store, &image)?;
    let base = store.root().join("macos-mounts");
    let intents = base.join("intents");
    let active = base.join("active");
    let path = intents.join(format!("{}.json", record::hash(mount_request.as_bytes())));
    let original = load(&path, references)?.ok_or_else(conflict)?;
    if original.request != mount_request || original.input != input {
        return Err(conflict());
    }
    if let Outcome::Cleanup {
        request: previous, ..
    } = &original.outcome
    {
        return if previous == request {
            Ok(Status::Released)
        } else {
            Err(conflict())
        };
    }
    let parent = record::parent(&active, &input, mount_request, original.version)?;
    let mount = parent.join("mount");
    let directories = [
        &base,
        &intents,
        &active,
        &parent,
        &store.root().join("qq-images"),
    ]
    .into_iter()
    .map(|p| Ok((p.to_path_buf(), acquire::directory(p)?)))
    .collect::<Result<Vec<_>>>()?;
    let transaction = store.transaction(deadline)?;
    let state_lock = transaction.lock_identity()?;
    if load(&path, references)?.as_ref() != Some(&original)
        || original.request != mount_request
        || original.input != input
        || identity(&acquire::directory(&parent)?)? != original.parent
    {
        return Err(conflict());
    }
    transaction.verify_lock()?;
    drop(transaction);
    let check = || {
        crate::core::deadline::check(deadline)?;
        lock.verify()?;
        for (path, held) in &directories {
            if identity(held)? != identity(&acquire::directory(path)?)? {
                return Err(conflict());
            }
        }
        input.verify_path(store)?;
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock {
            return Err(conflict());
        }
        transaction.verify_lock()?;
        lock.verify()
    };
    let guard_original = || {
        check()?;
        if load(&path, references)?.as_ref() != Some(&original) {
            return Err(conflict());
        }
        Ok(())
    };
    guard_original()?;
    let (fingerprint, receipt, fresh) = match &original.outcome {
        Outcome::Complete {
            fingerprint,
            receipt,
        } => (fingerprint.clone(), receipt.clone(), true),
        Outcome::Release {
            request: previous,
            fingerprint,
            receipt,
            complete,
        } => {
            if previous != request {
                return Err(conflict());
            }
            if *complete {
                return Ok(Status::Released);
            }
            (fingerprint.clone(), receipt.clone(), false)
        }
        _ => return Err(conflict()),
    };
    if transition::fingerprint(&image, deadline)? != fingerprint {
        return Err(conflict());
    }
    guard_original()?;
    let mut pending = original.clone();
    pending.outcome = Outcome::Release {
        request: request.into(),
        fingerprint: fingerprint.clone(),
        receipt: receipt.clone(),
        complete: false,
    };
    if fresh {
        // A prior external detach or reboot may already have removed the
        // authenticated attachment. Record release without inventing an effect.
        let observed = if receipt
            .require_gone(&image, &mount, deadline, &guard_original)
            .is_ok()
        {
            None
        } else {
            guard_original()?;
            let observed =
                mounted::Observation::inspect(&image, &mount, deadline, &guard_original)?;
            if observed.receipt()? != receipt {
                return Err(conflict());
            }
            Some(observed)
        };
        let pins = observed
            .as_ref()
            .map(|observed| observed.pin_devices(deadline, &guard_original))
            .transpose()?;
        guard_original()?;
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock
            || load(&path, references)?.as_ref() != Some(&original)
        {
            return Err(conflict());
        }
        save(&transaction, &lock, references, &path, &pending)?;
        drop(transaction);
        drop(observed);
        let guard = || {
            check()?;
            if load(&path, references)?.as_ref() != Some(&pending) {
                return Err(conflict());
            }
            Ok(())
        };
        guard()?;
        // Outcome is established independently below, including when an effect
        // succeeded but its direct command reported an error. Panic leaves Pending.
        if let Some(pins) = pins {
            let _result = operation(pins, &guard);
        }
    }
    let guard = || {
        check()?;
        if load(&path, references)?.as_ref() != Some(&pending) {
            return Err(conflict());
        }
        Ok(())
    };
    guard()?;
    if receipt
        .require_gone(&image, &mount, deadline, &guard)
        .is_err()
    {
        guard()?;
        return Ok(Status::Unknown);
    }
    if transition::fingerprint(&image, deadline)? != fingerprint {
        return Err(conflict());
    }
    guard()?;
    let transaction = store.transaction(deadline)?;
    if transaction.lock_identity()? != state_lock
        || load(&path, references)?.as_ref() != Some(&pending)
    {
        return Err(conflict());
    }
    let mut completed = pending;
    completed.outcome = Outcome::Release {
        request: request.into(),
        fingerprint,
        receipt,
        complete: true,
    };
    save(&transaction, &lock, references, &path, &completed)?;
    drop(transaction);
    check()?;
    if load(&path, references)?.as_ref() != Some(&completed) {
        return Err(conflict());
    }
    Ok(Status::Released)
}

#[cfg(test)]
mod tests;

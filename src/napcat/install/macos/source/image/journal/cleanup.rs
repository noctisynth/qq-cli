//! Retire only authenticated released empty directories; retain request tombstones.
use super::*;
use record::CleanupPhase;

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Complete,
}

pub fn cleanup(
    store: &Store,
    references: &References,
    request: &str,
    image: Image,
    deadline: Instant,
) -> Result<Status> {
    perform(store, references, request, image, deadline, &|| Ok(()))
}
pub(super) fn perform(
    store: &Store,
    references: &References,
    request: &str,
    image: Image,
    deadline: Instant,
    checkpoint: &dyn Fn() -> Result<()>,
) -> Result<Status> {
    crate::core::domain::request_key(request).map_err(|_| conflict())?;
    let lock = Lock::acquire(store.root(), "qq-mount.lock", deadline, "QQ_MOUNT_BUSY")?;
    let input = Input::inspect(store, &image)?;
    let base = store.root().join("macos-mounts");
    let intents = base.join("intents");
    let active = base.join("active");
    let path = intents.join(format!("{}.json", record::hash(request.as_bytes())));
    let directories = [&base, &intents, &active, &store.root().join("qq-images")]
        .into_iter()
        .map(|p| Ok((p.to_path_buf(), acquire::directory(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let transaction = store.transaction(deadline)?;
    let state_lock = transaction.lock_identity()?;
    let mut current = load(&path, references)?.ok_or_else(conflict)?;
    if current.request != request || current.input != input || current.version != 2 {
        return Err(conflict());
    }
    transaction.verify_lock()?;
    drop(transaction);
    let check = || {
        crate::core::deadline::check(deadline)?;
        lock.verify()?;
        input.verify_path(store)?;
        for (path, held) in &directories {
            if identity(held)? != identity(&acquire::directory(path)?)? {
                return Err(conflict());
            }
        }
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock {
            return Err(conflict());
        }
        transaction.verify_lock()?;
        lock.verify()
    };
    let parent = record::parent(&active, &input, request, current.version)?;
    let mount = parent.join("mount");
    loop {
        let guard = || {
            check()?;
            if load(&path, references)?.as_ref() != Some(&current) {
                return Err(conflict());
            }
            Ok(())
        };
        guard()?;
        let (release, fingerprint, receipt, phase) = match &current.outcome {
            Outcome::Release {
                request,
                fingerprint,
                receipt,
                complete: true,
            } => (request.clone(), fingerprint.clone(), receipt.clone(), None),
            Outcome::Cleanup {
                request,
                fingerprint,
                receipt,
                phase,
            } => (
                request.clone(),
                fingerprint.clone(),
                receipt.clone(),
                Some(*phase),
            ),
            _ => return Err(conflict()),
        };
        // Historical completion must not inspect or mutate a later attachment.
        if phase == Some(CleanupPhase::Complete) {
            return Ok(Status::Complete);
        }
        if transition::fingerprint(&image, deadline)? != fingerprint {
            return Err(conflict());
        }
        receipt.require_gone(&image, &mount, deadline, &guard)?;
        let next = match phase {
            None => {
                let held = original(&parent, current.parent)?.ok_or_else(conflict)?;
                original(&mount, current.mount.ok_or_else(conflict)?)?.ok_or_else(conflict)?;
                empty(&mount)?;
                guard()?;
                if identity(&held)? != identity(&acquire::directory(&parent)?)? {
                    return Err(conflict());
                }
                CleanupPhase::Mount
            }
            Some(CleanupPhase::Mount) => {
                let held = original(&parent, current.parent)?.ok_or_else(conflict)?;
                if let Some(root) = original(&mount, current.mount.ok_or_else(conflict)?)? {
                    empty(&mount)?;
                    guard()?;
                    if identity(&root)? != identity(&acquire::directory(&mount)?)? {
                        return Err(conflict());
                    }
                    rustix::fs::unlinkat(&held, "mount", rustix::fs::AtFlags::REMOVEDIR)
                        .map_err(|_| conflict())?;
                    checkpoint()?;
                }
                held.sync_all().map_err(|_| conflict())?;
                guard()?;
                if original(&mount, current.mount.ok_or_else(conflict)?)?.is_some() {
                    return Err(conflict());
                }
                CleanupPhase::Parent
            }
            Some(CleanupPhase::Parent) => {
                if let Some(held) = original(&parent, current.parent)? {
                    empty(&parent)?;
                    guard()?;
                    if identity(&held)? != identity(&acquire::directory(&parent)?)? {
                        return Err(conflict());
                    }
                    let active_fd = &directories[2].1;
                    rustix::fs::unlinkat(
                        active_fd,
                        parent.file_name().ok_or_else(conflict)?,
                        rustix::fs::AtFlags::REMOVEDIR,
                    )
                    .map_err(|_| conflict())?;
                    checkpoint()?;
                }
                directories[2].1.sync_all().map_err(|_| conflict())?;
                guard()?;
                if original(&parent, current.parent)?.is_some() {
                    return Err(conflict());
                }
                CleanupPhase::Complete
            }
            Some(CleanupPhase::Complete) => unreachable!(),
        };
        guard()?;
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock
            || load(&path, references)?.as_ref() != Some(&current)
        {
            return Err(conflict());
        }
        let mut next_record = current.clone();
        next_record.outcome = Outcome::Cleanup {
            request: release,
            fingerprint,
            receipt,
            phase: next,
        };
        save(&transaction, &lock, references, &path, &next_record)?;
        drop(transaction);
        current = next_record;
        checkpoint()?;
    }
}
fn original(path: &Path, expected: (u64, u64)) -> Result<Option<File>> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(conflict()),
        Ok(_) => {
            let held = acquire::directory(path)?;
            if identity(&held)? != expected {
                return Err(conflict());
            }
            Ok(Some(held))
        }
    }
}
fn empty(path: &Path) -> Result<()> {
    if std::fs::read_dir(path)
        .map_err(|_| conflict())?
        .next()
        .is_some()
    {
        return Err(conflict());
    }
    Ok(())
}

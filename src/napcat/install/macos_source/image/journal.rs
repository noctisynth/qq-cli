//! Durable one-shot mount dispatch. Ready is a read capability, never detach authority.
use super::*;
use crate::storage::{
    references::References,
    state::{Store, atomic_json, durable::Lock, private_dir},
};
use serde::{Deserialize, Serialize};
pub mod cleanup;
mod record;
pub mod release;
use record::{Input, Outcome, Record, identity, load, save};

pub enum Status {
    Unknown,
    Released,
    Ready(Box<Ready>),
}
pub struct Ready {
    image: Image,
    mount: PathBuf,
    parent: (u64, u64),
    fingerprint: String,
    receipt: mounted::Receipt,
    root: File,
}
impl Ready {
    /// Drop this mount reader before a durable release. Other Source/readers must also be dropped.
    pub fn into_image(self) -> Image {
        self.image
    }
    pub fn image(&self) -> &Image {
        &self.image
    }
    pub fn root(&self) -> &Path {
        &self.mount
    }
    pub fn revalidate(&self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        guard()?;
        if identity(&acquire::directory(
            self.mount.parent().ok_or_else(conflict)?,
        )?)? != self.parent
            || transition::fingerprint(&self.image, deadline)? != self.fingerprint
        {
            return Err(conflict());
        }
        let current = self
            .receipt
            .pin(&self.image, &self.mount, deadline, guard)?;
        if identity(&current)? != identity(&self.root)? {
            return Err(conflict());
        }
        guard()
    }
}

/// Only a fresh persisted Pending can call operation. A returned error or panic
/// leaves an unknown intent; repeating the request never calls operation again.
/// The callback must apply guard at its actual effect boundary.
pub fn attach(
    store: &Store,
    references: &References,
    request: &str,
    image: Image,
    deadline: Instant,
    operation: impl FnOnce(&Path, &Path, &dyn Fn() -> Result<()>) -> Result<()>,
) -> Result<Status> {
    crate::core::domain::request_key(request).map_err(|_| conflict())?;
    crate::core::deadline::check(deadline)?;
    let lock = Lock::acquire(store.root(), "qq-mount.lock", deadline, "QQ_MOUNT_BUSY")?;
    let input = Input::inspect(store, &image)?;
    let base = store.root().join("macos-mounts");
    let intents = base.join("intents");
    let active = base.join("active");
    for path in [&base, &intents, &active] {
        private_dir(path)?;
        acquire::directory(path)?;
    }
    let directories = [&base, &intents, &active, &store.root().join("qq-images")]
        .into_iter()
        .map(|p| Ok((p.to_path_buf(), acquire::directory(p)?)))
        .collect::<Result<Vec<_>>>()?;
    let key = record::hash(request.as_bytes());
    let path = intents.join(format!("{key}.json"));
    let transaction = store.transaction(deadline)?;
    let state_lock = transaction.lock_identity()?;
    let existing = load(&path, references)?;
    let parent = record::parent(
        &active,
        &input,
        request,
        existing.as_ref().map_or(2, |r| r.version),
    )?;
    let mount = parent.join("mount");
    if existing.is_none() {
        record::exclusive(&intents, references, request, &input)?;
    }
    transaction.verify_lock()?;
    drop(transaction);
    // A full byte/metadata check is deliberately outside the state transaction.
    let check = || {
        crate::core::deadline::check(deadline)?;
        lock.verify()?;
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
    if let Some(record) = existing {
        if record.request != request || record.input != input {
            return Err(conflict());
        }
        let guard = || {
            check()?;
            if load(&path, references)?.as_ref() != Some(&record) {
                return Err(conflict());
            }
            Ok(())
        };
        guard()?;
        return match &record.outcome {
            Outcome::Pending | Outcome::Unknown => Ok(Status::Unknown),
            Outcome::Cleanup { .. } => Ok(Status::Released),
            Outcome::Release { complete, .. } => Ok(if *complete {
                Status::Released
            } else {
                Status::Unknown
            }),
            Outcome::Complete {
                fingerprint,
                receipt,
            } => {
                let root = receipt.pin(&image, &mount, deadline, &guard)?;
                let ready = Ready {
                    image,
                    mount,
                    parent: record.parent,
                    fingerprint: fingerprint.clone(),
                    receipt: receipt.clone(),
                    root,
                };
                ready.revalidate(deadline, &guard)?;
                Ok(Status::Ready(Box::new(ready)))
            }
        };
    }
    image.revalidate(deadline)?;
    mounted::require_absent(image.path(), &mount, deadline, &check)?;
    let before = transition::fingerprint(&image, deadline)?;
    let transition = transition::MountTransition::prepare(image, deadline, &check)?;
    check()?;
    record::create_directory(&parent)?;
    record::create_directory(&mount)?;
    let parent_id = identity(&acquire::directory(&parent)?)?;
    let pending = Record {
        version: 2,
        request: request.into(),
        input,
        before,
        parent: parent_id,
        mount: Some(identity(&acquire::directory(&mount)?)?),
        outcome: Outcome::Pending,
    };
    {
        let transaction = store.transaction(deadline)?;
        if transaction.lock_identity()? != state_lock || load(&path, references)?.is_some() {
            return Err(conflict());
        }
        save(&transaction, &lock, references, &path, &pending)?;
    }
    let guard = || {
        check()?;
        if load(&path, references)?.as_ref() != Some(&pending)
            || identity(&acquire::directory(&parent)?)? != parent_id
        {
            return Err(conflict());
        }
        pending.input.verify_path(store)?;
        Ok(())
    };
    let result: Result<Ready> = (|| {
        transition.verify_before(deadline, &guard)?;
        mounted::require_absent(transition.path(), &mount, deadline, &guard)?;
        guard()?;
        operation(transition.path(), &mount, &guard)?;
        let image = transition.finish(deadline, &guard)?;
        let receipt = mounted::Observation::inspect(&image, &mount, deadline, &guard)?.receipt()?;
        let fingerprint = transition::fingerprint(&image, deadline)?;
        let root = receipt.pin(&image, &mount, deadline, &guard)?;
        let ready = Ready {
            image,
            mount,
            parent: parent_id,
            fingerprint,
            receipt,
            root,
        };
        ready.revalidate(deadline, &guard)?;
        Ok(ready)
    })();
    guard()?;
    let transaction = store.transaction(deadline)?;
    if transaction.lock_identity()? != state_lock
        || load(&path, references)?.as_ref() != Some(&pending)
    {
        return Err(conflict());
    }
    let mut completed = pending.clone();
    completed.outcome = match &result {
        Ok(ready) => Outcome::Complete {
            fingerprint: ready.fingerprint.clone(),
            receipt: ready.receipt.clone(),
        },
        Err(_) => Outcome::Unknown,
    };
    save(&transaction, &lock, references, &path, &completed)?;
    drop(transaction);
    check()?;
    match result {
        Ok(ready) => {
            ready.revalidate(deadline, &|| {
                check()?;
                if load(&path, references)?.as_ref() != Some(&completed) {
                    return Err(conflict());
                }
                Ok(())
            })?;
            Ok(Status::Ready(Box::new(ready)))
        }
        Err(_) => Ok(Status::Unknown),
    }
}
fn conflict() -> AppError {
    AppError::new(
        "QQ_MOUNT_CONFLICT",
        "Mount intent or original inputs changed; retain the scene without repeating or detaching it.",
        4,
    )
}

#[cfg(test)]
mod tests;

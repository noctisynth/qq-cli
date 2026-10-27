//! Reuse only an authenticated QQ profile from this instance's stopped
//! predecessor. The signed l2 declaration is flattened to the original owner;
//! no user data is moved, copied, repaired or discovered by path.
use super::*;

pub(super) struct Profile {
    root: PathBuf,
    home: PathBuf,
    snapshot: fs::Snapshot,
}
impl Profile {
    pub(super) fn home(&self) -> &Path {
        &self.home
    }
    pub(super) fn verify(&self, current: &Path, deadline: Instant) -> Result<()> {
        if self.root != current {
            self.snapshot.verify(&self.root, false, deadline)?;
        }
        Ok(())
    }
}

pub(super) fn restore(
    root: &Path,
    record: &Record,
    refs: &References,
    deadline: Instant,
) -> Result<Profile> {
    let (owner_root, owner) = if let Some(reference) = &record.profile {
        let owner = authenticate(refs, reference)?;
        if owner.profile.is_some()
            || owner.target.instance != record.target.instance
            || owner.target.run == record.target.run
        {
            return Err(invalid());
        }
        let id = crate::napcat::runtime::runtime_store::identity(
            &owner.target.instance,
            &owner.target.run,
        )?;
        (root.parent().ok_or_else(invalid)?.join(id), owner)
    } else {
        (root.to_owned(), record.clone())
    };
    let Outcome::Complete(snapshot) = owner.outcome else {
        return Err(invalid());
    };
    snapshot.verify(&owner_root, false, deadline)?;
    Ok(Profile {
        home: owner_root.join("profile-home"),
        root: owner_root,
        snapshot,
    })
}

pub(super) fn inherit(
    tx: &Transaction<'_>,
    refs: &References,
    instance: &Instance,
    deadline: Instant,
) -> Result<Option<String>> {
    use crate::napcat::runtime::{generation, macos::stop};
    let Some((source, installation)) = generation::macos_predecessor(tx.store(), refs, instance)?
    else {
        return Ok(None);
    };
    if source.id != instance.id || source.run_id == instance.run_id {
        return Err(invalid());
    }
    stop::require_completed(tx.store(), refs, &source, &installation)?;
    let target = Target {
        instance: source.id,
        run: source.run_id,
        installation: installation.id,
        digest: installation.digest,
    };
    let (path, root) = paths(tx.store(), &target)?;
    let (record, reference) = load_bound(&path, refs)?.ok_or_else(invalid)?;
    if record.target != target || !matches!(record.outcome, Outcome::Complete(_)) {
        return Err(invalid());
    }
    // Revalidate the old directory identities before publishing the new intent.
    let Preparation::Ready(_) = layout(root, &record, refs, deadline)? else {
        return Err(invalid());
    };
    tx.verify_lock()?;
    Ok(Some(record.profile.unwrap_or(reference)))
}

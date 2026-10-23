//! Verify a re-sign transition on an already sealed application tree.
use super::*;

impl SigningPlan {
    /// Compare an original snapshot with a freshly observed signed tree.
    /// This is read-only: it grants no mutation authority and does not verify
    /// signature validity or entitlements. Missing original seals are rejected.
    pub fn verify_transition(
        &self,
        before: &Inventory,
        after: &Inventory,
        deadline: Instant,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if before.root != self.root
            || after.root != self.root
            || before.fingerprint != self.fingerprint
            || before.entries.len() != after.entries.len()
        {
            return Err(invalid());
        }
        changes(&self.targets, before, after, deadline)?;
        self.verify_payloads(after, deadline)
    }
}

impl SigningTarget {
    pub(super) fn verify_step(
        &self,
        before: &Inventory,
        after: &Inventory,
        deadline: Instant,
    ) -> Result<()> {
        changes(std::slice::from_ref(self), before, after, deadline)?;
        let Content::File { size, .. } = after
            .entries
            .get(&self.executable)
            .ok_or_else(invalid)?
            .content
        else {
            return Err(invalid());
        };
        let mut file = after.signing_file(&self.executable)?;
        if ResignPayload::inspect(&mut file, size, deadline)? != self.payload {
            return Err(invalid());
        }
        after.verify_stamps(deadline)
    }
}

fn changes(
    targets: &[SigningTarget],
    before: &Inventory,
    after: &Inventory,
    deadline: Instant,
) -> Result<()> {
    if before.root != after.root || before.entries.len() != after.entries.len() {
        return Err(invalid());
    }
    let mut mutable = BTreeSet::new();
    for target in targets {
        mutable.insert(target.executable.as_path());
        if let Some(seal) = &target.seal {
            if !matches!(
                before.entries.get(seal).map(|e| &e.content),
                Some(Content::File { .. })
            ) {
                return Err(invalid());
            }
            mutable.insert(seal.as_path());
        }
    }
    let parents: BTreeSet<_> = mutable.iter().filter_map(|p| p.parent()).collect();
    for (path, old) in &before.entries {
        crate::core::deadline::check(deadline)?;
        let new = after.entries.get(path).ok_or_else(invalid)?;
        let a = &before.stamps[path];
        let b = &after.stamps[path];
        if mutable.contains(path.as_path()) {
            if !matches!(old.content, Content::File { .. })
                || !matches!(new.content, Content::File { .. })
                || old.mode != new.mode
                || old.flags != new.flags
                || old.acl != new.acl
                || old.attributes != new.attributes
                || a.device != b.device
                || a.uid != b.uid
                || a.gid != b.gid
                || a.links != b.links
                || a.mode != b.mode
            {
                return Err(invalid());
            }
        } else if parents.contains(path.as_path()) {
            if old != new
                || !matches!(old.content, Content::Directory)
                || a.device != b.device
                || a.inode != b.inode
                || a.uid != b.uid
                || a.gid != b.gid
                || a.links != b.links
                || a.mode != b.mode
                || a.flags != b.flags
                || a.size != b.size
            {
                return Err(invalid());
            }
        } else if old != new || a != b {
            return Err(invalid());
        }
    }
    Ok(())
}

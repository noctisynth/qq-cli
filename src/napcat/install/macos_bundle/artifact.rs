//! A persistent observation of a completed live-copy signing transaction.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Artifact {
    fingerprint: String,
    device: u64,
    inode: u64,
    parent_device: u64,
    parent_inode: u64,
    uid: u32,
}
impl Inventory {
    pub(crate) fn finish_deployment(
        self,
        expected: &Path,
        deadline: Instant,
        guard: &impl Fn() -> Result<()>,
    ) -> Result<Artifact> {
        if !self.signed || self.root != expected {
            return Err(invalid());
        }
        let owner = self.owner.as_ref().ok_or_else(invalid)?;
        guard()?;
        owner.verify()?;
        self.revalidate(deadline)?;
        guard()?;
        owner.verify()?;
        let root = &self.stamps[Path::new("")];
        let parent = owner.file.metadata().map_err(|_| invalid())?;
        Ok(Artifact {
            fingerprint: self.fingerprint.clone(),
            device: root.device,
            inode: root.inode,
            parent_device: parent.dev(),
            parent_inode: parent.ino(),
            uid: root.uid,
        })
    }
}
impl Artifact {
    /// Never restores live-copy mutation authority. The authenticated journal
    /// supplies this record; the caller derives the fixed instance path.
    pub(crate) fn verify(&self, root: &Path, deadline: Instant) -> Result<()> {
        let parent_path = root.parent().ok_or_else(invalid)?;
        reject_links(parent_path)?;
        let parent = open(parent_path, None, true, false)?;
        let meta = parent.metadata().map_err(|_| invalid())?;
        if meta.dev() != self.parent_device
            || meta.ino() != self.parent_inode
            || meta.uid() != self.uid
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        crate::native::macos_acl::require_absent(parent.as_fd())?;
        let inventory = Inventory::inspect(root, deadline)?;
        let stamp = &inventory.stamps[Path::new("")];
        if stamp.device != self.device
            || stamp.inode != self.inode
            || stamp.uid != self.uid
            || inventory.fingerprint != self.fingerprint
            || inventory.stamps.values().any(|stamp| stamp.uid != self.uid)
        {
            return Err(invalid());
        }
        let current = std::fs::symlink_metadata(parent_path).map_err(|_| invalid())?;
        if Stamp::from(&current) != Stamp::from(&meta) {
            return Err(invalid());
        }
        inventory.verify_stamps(deadline)
    }
}

//! Authenticate the bounded metadata transition around a separately authorized
//! mount. This snapshot neither dispatches nor proves a mount operation.
use super::*;
use crate::napcat::install::macos::bundle::attributes::{Attributes, Budget};
use std::os::{fd::AsFd, macos::fs::MetadataExt as _};

#[derive(PartialEq, Eq, serde::Serialize)]
struct Protection {
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
    flags: u32,
    born: (i64, i64),
    acl: [u8; 32],
}
fn protection(file: &File) -> Result<Protection> {
    let metadata = file.metadata().map_err(|_| invalid())?;
    Ok(Protection {
        uid: metadata.uid(),
        gid: metadata.gid(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        flags: metadata.st_flags(),
        born: (metadata.st_birthtime(), metadata.st_birthtime_nsec()),
        acl: crate::native::macos_acl::digest(file.as_fd())?,
    })
}

pub(super) fn fingerprint(image: &Image, deadline: Instant) -> Result<String> {
    image.revalidate(deadline)?;
    let protection = protection(&image.file)?;
    let attributes = Attributes::read(&image.file, &mut Budget::default(), deadline)?;
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-image-snapshot-v1\0");
    hash.update(
        serde_json::to_vec(&(&image.path, &image.stamp, &image.digest, protection))
            .map_err(|_| invalid())?,
    );
    attributes.digest(&mut hash);
    if stamp(&image.file)? != image.stamp {
        return Err(invalid());
    }
    crate::core::deadline::check(deadline)?;
    Ok(format!("{:x}", hash.finalize()))
}

pub struct MountTransition {
    before: Image,
    protection: Protection,
    attributes: Attributes,
}
impl MountTransition {
    pub fn prepare(
        image: Image,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        guard()?;
        image.revalidate(deadline)?;
        let protection = protection(&image.file)?;
        let attributes = Attributes::read(&image.file, &mut Budget::default(), deadline)?;
        if stamp(&image.file)? != image.stamp || !attributes.verifies_image_mount(&attributes) {
            return Err(invalid());
        }
        guard()?;
        crate::core::deadline::check(deadline)?;
        Ok(Self {
            before: image,
            protection,
            attributes,
        })
    }

    pub fn path(&self) -> &Path {
        self.before.path()
    }

    /// Recheck the unchanged original immediately before a separately
    /// authorized attach effect. Once metadata changes, only finish may accept
    /// its narrowly allowed transition; this check never refreshes the baseline.
    pub fn verify_before(&self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        guard()?;
        self.before.revalidate(deadline)?;
        if protection(&self.before.file)? != self.protection
            || Attributes::read(&self.before.file, &mut Budget::default(), deadline)?
                != self.attributes
            || stamp(&self.before.file)? != self.before.stamp
        {
            return Err(invalid());
        }
        guard()?;
        crate::core::deadline::check(deadline)
    }

    /// Consumes the original snapshot, checks all bytes again, and only accepts
    /// the same file with its bounded system checksum-cache metadata updated.
    /// No writes, repairs, replay, detach authority or mount receipt are created.
    pub fn finish(self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<Image> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        let before = self.before;
        let after =
            Image::inspect_expected(before.path(), before.stamp.size, &before.digest, deadline)?;
        let same_input = before.stamp.device == after.stamp.device
            && before.stamp.inode == after.stamp.inode
            && before.stamp.size == after.stamp.size
            && before.stamp.modified == after.stamp.modified;
        if !same_input
            || before.quarantine != after.quarantine
            || stamp(&before.file)? != after.stamp
            || protection(&before.file)? != self.protection
            || protection(&after.file)? != self.protection
        {
            return Err(invalid());
        }
        let attributes = Attributes::read(&before.file, &mut Budget::default(), deadline)?;
        if !self.attributes.verifies_image_mount(&attributes) || stamp(&before.file)? != after.stamp
        {
            return Err(invalid());
        }
        after.revalidate(deadline)?;
        guard()?;
        crate::core::deadline::check(deadline)?;
        Ok(after)
    }
}

#[cfg(test)]
mod tests;

//! Hash every filesystem-reported xattr without exposing values or following links.
use super::{field, invalid};
use crate::core::error::Result;
use crate::native::macos_xattr;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::os::fd::AsFd;
use std::time::Instant;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Attributes(BTreeMap<Vec<u8>, (u64, [u8; 32])>);

#[derive(Default)]
pub(super) struct Budget {
    ordinary: u64,
    forks: u64,
}

impl Budget {
    fn add(&mut self, name: &[u8], size: u64) -> Result<()> {
        let fork = name == b"com.apple.ResourceFork";
        self.ordinary = self
            .ordinary
            .checked_add(name.len() as u64)
            .and_then(|v| v.checked_add(if fork { 0 } else { size }))
            .ok_or_else(invalid)?;
        if self.ordinary > 64 * 1024 * 1024 {
            return Err(invalid());
        }
        if fork {
            self.forks = self.forks.checked_add(size).ok_or_else(invalid)?;
            if self.forks > 4 * 1024 * 1024 * 1024 {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

fn names(file: &File) -> Result<BTreeSet<Vec<u8>>> {
    let mut bytes = vec![0; 64 * 1024];
    let count = macos_xattr::list(file.as_fd(), bytes.as_mut_slice()).map_err(|_| invalid())?;
    if count > bytes.len() {
        return Err(invalid());
    }
    bytes.truncate(count);
    let mut names = BTreeSet::new();
    if !bytes.is_empty() {
        if bytes.last() != Some(&0) {
            return Err(invalid());
        }
        for name in bytes[..bytes.len() - 1].split(|b| *b == 0) {
            if name.is_empty()
                || name.len() > 255
                || !names.insert(name.to_vec())
                || names.len() > 256
            {
                return Err(invalid());
            }
        }
    }
    Ok(names)
}

impl Attributes {
    pub(super) fn account(&self, budget: &mut Budget) -> Result<()> {
        for (name, (size, _)) in &self.0 {
            budget.add(name, *size)?;
        }
        Ok(())
    }
    /// Kernel write-open decompression removes decmpfs and may remove its
    /// resource fork. A surviving fork must be byte-identical; all other
    /// attributes (especially quarantine) must remain exactly unchanged.
    pub(super) fn verifies_decompression(&self, after: &Self) -> bool {
        let compression = b"com.apple.decmpfs".as_slice();
        let fork = b"com.apple.ResourceFork".as_slice();
        self.0.contains_key(compression)
            && !after.0.contains_key(compression)
            && self.0.iter().all(|(name, value)| {
                name == compression
                    || (name == fork && !after.0.contains_key(fork))
                    || after.0.get(name) == Some(value)
            })
            && after
                .0
                .iter()
                .all(|(name, value)| self.0.get(name) == Some(value))
    }

    /// fcopyfile normalizes quarantine through its quarantine API. Preserve
    /// the authenticated source bytes instead; never remove/clear quarantine.
    pub(super) fn copy_quarantine(&self, source: &File, destination: &File) -> Result<()> {
        let Some((size, expected)) = self.0.get(b"com.apple.quarantine".as_slice()) else {
            return Ok(());
        };
        let mut value = vec![0; *size as usize + 1];
        let read = macos_xattr::read(source.as_fd(), c"com.apple.quarantine", &mut value)
            .map_err(|_| invalid())?;
        if read as u64 != *size {
            return Err(invalid());
        }
        value.truncate(read);
        let actual: [u8; 32] = Sha256::digest(&value).into();
        if &actual != expected {
            return Err(invalid());
        }
        rustix::fs::fsetxattr(
            destination,
            "com.apple.quarantine",
            &value,
            rustix::fs::XattrFlags::empty(),
        )
        .map_err(|_| invalid())
    }

    pub(super) fn count(&self) -> usize {
        self.0.len()
    }
    pub(super) fn read(file: &File, budget: &mut Budget, deadline: Instant) -> Result<Self> {
        let listed = names(file)?;
        let mut values = BTreeMap::new();
        for name in &listed {
            crate::core::deadline::check(deadline)?;
            let key = std::ffi::CString::new(name.as_slice()).map_err(|_| invalid())?;
            let size = macos_xattr::read(file.as_fd(), &key, &mut []).map_err(|_| invalid())?;
            let fork = name == b"com.apple.ResourceFork";
            if size as u64
                > if fork {
                    2 * 1024 * 1024 * 1024
                } else {
                    16 * 1024 * 1024
                }
            {
                return Err(invalid());
            }
            budget.add(name, size as u64)?;
            if fork {
                let mut hash = Sha256::new();
                let mut bytes = vec![0; 65536];
                let mut offset = 0;
                while offset < size {
                    crate::core::deadline::check(deadline)?;
                    let count = (size - offset).min(bytes.len());
                    let read = macos_xattr::read_at(
                        file.as_fd(),
                        &key,
                        &mut bytes[..count],
                        offset as u32,
                    )
                    .map_err(|_| invalid())?;
                    if read != count {
                        return Err(invalid());
                    }
                    hash.update(&bytes[..read]);
                    offset += read;
                }
                if macos_xattr::read(file.as_fd(), &key, &mut []).map_err(|_| invalid())? != size {
                    return Err(invalid());
                }
                values.insert(name.clone(), (size as u64, hash.finalize().into()));
                continue;
            }
            let mut bytes = vec![0; size + 1];
            let read = macos_xattr::read(file.as_fd(), &key, bytes.as_mut_slice())
                .map_err(|_| invalid())?;
            if read != size {
                return Err(invalid());
            }
            values.insert(
                name.clone(),
                (size as u64, Sha256::digest(&bytes[..size]).into()),
            );
        }
        if names(file)? != listed {
            return Err(invalid());
        }
        Ok(Self(values))
    }
    pub(super) fn digest(&self, hash: &mut Sha256) {
        hash.update((self.0.len() as u64).to_le_bytes());
        for (name, (size, digest)) in &self.0 {
            field(hash, name);
            hash.update(size.to_le_bytes());
            hash.update(digest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn decompression_accepts_only_removal_of_compression_metadata() {
        let snapshot = |items: &[(&[u8], &[u8])]| {
            Attributes(
                items
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.to_vec(),
                            (value.len() as u64, Sha256::digest(value).into()),
                        )
                    })
                    .collect(),
            )
        };
        let compression = (b"com.apple.decmpfs".as_slice(), b"compression".as_slice());
        let fork = (b"com.apple.ResourceFork".as_slice(), b"fork".as_slice());
        let quarantine = (b"com.apple.quarantine".as_slice(), b"keep".as_slice());
        let before = snapshot(&[compression, fork, quarantine]);
        assert!(before.verifies_decompression(&snapshot(&[fork, quarantine])));
        assert!(before.verifies_decompression(&snapshot(&[quarantine])));
        for items in [
            vec![compression, quarantine],
            vec![fork],
            vec![quarantine, (fork.0, b"changed")],
            vec![(quarantine.0, b"changed")],
            vec![quarantine, (b"new attribute", b"new")],
        ] {
            assert!(!before.verifies_decompression(&snapshot(&items)));
        }
        assert!(!snapshot(&[fork, quarantine]).verifies_decompression(&snapshot(&[quarantine])));
    }

    #[test]
    fn link_descriptor_attributes_do_not_follow_target_attributes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let target = root.join("target");
        let link = root.join("link");
        std::fs::write(&target, b"synthetic").unwrap();
        rustix::fs::setxattr(
            &target,
            "org.qq-cli.target-only",
            b"private synthetic value",
            rustix::fs::XattrFlags::empty(),
        )
        .unwrap();
        std::os::unix::fs::symlink("target", &link).unwrap();
        let target = super::super::open(&target, None, false, false).unwrap();
        let link = super::super::open(&link, None, false, true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut budget = Budget::default();
        let target = Attributes::read(&target, &mut budget, deadline).unwrap();
        let link = Attributes::read(&link, &mut budget, deadline).unwrap();
        assert!(target.0.contains_key(b"org.qq-cli.target-only".as_slice()));
        assert!(!link.0.contains_key(b"org.qq-cli.target-only".as_slice()));
    }

    #[test]
    fn link_descriptor_acl_does_not_follow_target_acl() {
        use std::os::fd::AsFd;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let target_path = root.join("target");
        let link_path = root.join("link");
        std::fs::write(&target_path, b"synthetic").unwrap();
        std::os::unix::fs::symlink("target", &link_path).unwrap();
        let target = super::super::open(&target_path, None, false, false).unwrap();
        let link = super::super::open(&link_path, None, false, true).unwrap();
        let digest = crate::native::macos_acl::digest;
        let initial = digest(link.as_fd()).unwrap();
        assert_eq!(initial, digest(target.as_fd()).unwrap());
        assert!(
            std::process::Command::new("/bin/chmod")
                .args(["+a", "everyone allow read"])
                .arg(&target_path)
                .status()
                .unwrap()
                .success()
        );
        assert_ne!(initial, digest(target.as_fd()).unwrap());
        assert_eq!(initial, digest(link.as_fd()).unwrap());
        assert!(
            std::process::Command::new("/bin/chmod")
                .args(["-h", "+a", "everyone deny write"])
                .arg(&link_path)
                .status()
                .unwrap()
                .success()
        );
        assert_ne!(initial, digest(link.as_fd()).unwrap());
    }
}

//! Content and metadata inventory for copy verification. Not a signature,
//! source authorization or an instruction to execute files.
mod attributes;
mod copy;
mod deploy;
mod harden;
mod signing;
use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;
use sha2::{Digest, Sha256};
pub use signing::supervisor_entry;
pub use signing::{BundleRole, SigningPlan, SigningTarget};
use std::collections::BTreeMap;
use std::fs::{File, Metadata, OpenOptions};
use std::io::Read;
use std::os::fd::AsFd;
use std::os::macos::fs::MetadataExt as MacMetadataExt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;

const MAX_ENTRIES: usize = 20_000;
const MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_FILE: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq)]
enum Content {
    Directory,
    File { size: u64, hash: [u8; 32] },
    Link(PathBuf),
}
#[derive(Clone, PartialEq, Eq)]
struct Entry {
    mode: u32,
    flags: u32,
    acl: [u8; 32],
    content: Content,
    attributes: attributes::Attributes,
}
#[derive(PartialEq, Eq)]
struct Stamp {
    flags: u32,
    device: u64,
    inode: u64,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl From<&Metadata> for Stamp {
    fn from(meta: &Metadata) -> Self {
        Self {
            flags: meta.st_flags(),
            device: meta.dev(),
            inode: meta.ino(),
            size: meta.len(),
            mode: meta.mode(),
            uid: meta.uid(),
            gid: meta.gid(),
            links: meta.nlink(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}

/// Keeps private relative names out of Debug/Serialize and public CLI output.
pub struct Inventory {
    owner: Option<copy::Parent>,
    root: PathBuf,
    entries: BTreeMap<PathBuf, Entry>,
    stamps: BTreeMap<PathBuf, Stamp>,
    bytes: u64,
    attribute_budget: attributes::Budget,
    fingerprint: String,
    #[cfg(test)]
    content_bytes_read: u64,
}
impl Inventory {
    pub fn inspect(root: &Path, deadline: Instant) -> Result<Self> {
        Self::inspect_using(root, deadline, None)
    }
    fn inspect_using(root: &Path, deadline: Instant, cache: Option<&Self>) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        reject_links(root)?;
        if !root.is_absolute()
            || root.extension().is_none_or(|ext| ext != "app")
            || root.canonicalize().map_err(|_| invalid())?.as_os_str() != root.as_os_str()
        {
            return Err(invalid());
        }
        let meta = std::fs::symlink_metadata(root).map_err(|_| invalid())?;
        if !meta.is_dir() {
            return Err(invalid());
        }
        let mut value = Self {
            owner: None,
            root: root.into(),
            entries: BTreeMap::new(),
            stamps: BTreeMap::new(),
            bytes: 0,
            attribute_budget: attributes::Budget::default(),
            fingerprint: String::new(),
            #[cfg(test)]
            content_bytes_read: 0,
        };
        value.walk(Path::new(""), meta.dev(), 0, (deadline, cache), None)?;
        value.verify_stamps(deadline)?;
        value.fingerprint = value.digest()?;
        Ok(value)
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn total_bytes(&self) -> u64 {
        self.bytes
    }
    /// Includes the root directory itself.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    pub fn extended_attribute_count(&self) -> usize {
        self.entries
            .values()
            .map(|entry| entry.attributes.count())
            .sum()
    }
    /// Source observation only. A private deployment must address these modes;
    /// recording a valid signed source is not approval to execute it in place.
    pub fn externally_writable_files(&self) -> usize {
        self.entries
            .values()
            .filter(|entry| {
                matches!(entry.content, Content::File { .. }) && entry.mode & 0o022 != 0
            })
            .count()
    }

    /// Reinspect the same root and require original identities as well as content.
    pub fn revalidate(&self, deadline: Instant) -> Result<()> {
        self.verify_stamps(deadline)?;
        let current = Self::inspect(&self.root, deadline)?;
        if current.entries != self.entries || current.stamps != self.stamps {
            return Err(invalid());
        }
        Ok(())
    }
    /// Copy comparison deliberately excludes source inode/UID/timestamps.
    /// Code signatures and deployment permission policy require separate checks.
    pub fn verify_copy(&self, copy: &Path, deadline: Instant) -> Result<Self> {
        if copy.starts_with(&self.root) || self.root.starts_with(copy) {
            return Err(invalid());
        }
        let candidate = Self::inspect(copy, deadline)?;
        if candidate.entries != self.entries {
            return Err(invalid());
        }
        self.verify_stamps(deadline)?;
        Ok(candidate)
    }
    fn walk(
        &mut self,
        relative: &Path,
        device: u64,
        depth: usize,
        (deadline, cache): (Instant, Option<&Self>),
        parent: Option<&File>,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if depth > 64 || self.entries.len() >= MAX_ENTRIES {
            return Err(invalid());
        }
        let text = relative.to_str().ok_or_else(invalid)?;
        if text.len() > 4096 || text.chars().any(char::is_control) {
            return Err(invalid());
        }
        let path = self.root.join(relative);
        let meta = std::fs::symlink_metadata(&path).map_err(|_| invalid())?;
        if meta.dev() != device {
            return Err(invalid());
        }
        let stamp = Stamp::from(&meta);
        if !meta.is_file() && !meta.is_dir() && !meta.file_type().is_symlink() {
            return Err(invalid());
        }
        let mut handle = open(&path, parent, meta.is_dir(), meta.file_type().is_symlink())?;
        if Stamp::from(&handle.metadata().map_err(|_| invalid())?) != stamp {
            return Err(invalid());
        }
        let cached = cache
            .filter(|old| old.root == self.root && old.stamps.get(relative) == Some(&stamp))
            .and_then(|old| old.entries.get(relative));
        let content = if let Some(entry) = cached {
            if let Content::File { size, .. } = entry.content {
                self.bytes = self.bytes.checked_add(size).ok_or_else(invalid)?;
                if self.bytes > MAX_BYTES {
                    return Err(invalid());
                }
            }
            entry.content.clone()
        } else if meta.file_type().is_symlink() {
            let target = std::fs::read_link(&path).map_err(|_| invalid())?;
            let text = target.to_str().ok_or_else(invalid)?;
            if target.is_absolute()
                || text.is_empty()
                || text.len() > 4096
                || text.chars().any(char::is_control)
            {
                return Err(invalid());
            }
            let resolved = path.canonicalize().map_err(|_| invalid())?;
            if !resolved.starts_with(&self.root) {
                return Err(invalid());
            }
            Content::Link(target)
        } else {
            if meta.mode() & 0o7000 != 0 || (meta.is_dir() && meta.mode() & 0o022 != 0) {
                return Err(invalid());
            }
            if meta.is_dir() {
                Content::Directory
            } else if meta.is_file() && meta.nlink() == 1 {
                self.bytes = self.bytes.checked_add(meta.len()).ok_or_else(invalid)?;
                if meta.len() > MAX_FILE || self.bytes > MAX_BYTES {
                    return Err(invalid());
                }
                let mut hash = Sha256::new();
                let mut bytes = vec![0; 65536];
                let mut total = 0;
                loop {
                    crate::core::deadline::check(deadline)?;
                    let count = handle.read(&mut bytes).map_err(|_| invalid())?;
                    if count == 0 {
                        break;
                    }
                    total += count as u64;
                    #[cfg(test)]
                    {
                        self.content_bytes_read += count as u64;
                    }
                    if total > meta.len() {
                        return Err(invalid());
                    }
                    hash.update(&bytes[..count]);
                }
                if total != meta.len() {
                    return Err(invalid());
                }
                Content::File {
                    size: total,
                    hash: hash.finalize().into(),
                }
            } else {
                return Err(invalid());
            }
        };
        let is_directory = matches!(content, Content::Directory);
        let (attributes, acl) = if let Some(entry) = cached {
            entry.attributes.account(&mut self.attribute_budget)?;
            (entry.attributes.clone(), entry.acl)
        } else {
            (
                attributes::Attributes::read(&handle, &mut self.attribute_budget, deadline)?,
                crate::native::macos_acl::digest(handle.as_fd())?,
            )
        };
        self.entries.insert(
            relative.into(),
            Entry {
                mode: meta.mode() & 0o777,
                flags: meta.st_flags(),
                acl,
                content,
                attributes,
            },
        );
        self.stamps.insert(relative.into(), stamp);
        if is_directory {
            let directory = handle;
            if Stamp::from(&directory.metadata().map_err(|_| invalid())?) != self.stamps[relative] {
                return Err(invalid());
            }
            // Enumerate/open through held directories; never follow a parent
            // path replacement into another tree while reading file contents.
            for child in rustix::fs::Dir::read_from(&directory).map_err(|_| invalid())? {
                let child = child.map_err(|_| invalid())?;
                let name = child.file_name().to_bytes();
                if name == b"." || name == b".." {
                    continue;
                }
                self.walk(
                    &relative.join(std::ffi::OsStr::from_bytes(name)),
                    device,
                    depth + 1,
                    (deadline, cache),
                    Some(&directory),
                )?;
            }
        }
        Ok(())
    }
    fn verify_stamps(&self, deadline: Instant) -> Result<()> {
        reject_links(&self.root)?;
        for (relative, expected) in &self.stamps {
            crate::core::deadline::check(deadline)?;
            let current =
                std::fs::symlink_metadata(self.root.join(relative)).map_err(|_| invalid())?;
            if Stamp::from(&current) != *expected {
                return Err(invalid());
            }
        }
        Ok(())
    }
    fn digest(&self) -> Result<String> {
        let mut hash = Sha256::new();
        hash.update(b"qq-cli-macos-app-tree-v4\0");
        for (path, entry) in &self.entries {
            field(&mut hash, path.to_str().ok_or_else(invalid)?.as_bytes());
            hash.update(entry.mode.to_le_bytes());
            hash.update(entry.flags.to_le_bytes());
            hash.update(entry.acl);
            entry.attributes.digest(&mut hash);
            match &entry.content {
                Content::Directory => hash.update([0]),
                Content::File {
                    size,
                    hash: content,
                } => {
                    hash.update([1]);
                    hash.update(size.to_le_bytes());
                    hash.update(content);
                }
                Content::Link(target) => {
                    hash.update([2]);
                    field(&mut hash, target.to_str().ok_or_else(invalid)?.as_bytes());
                }
            }
        }
        Ok(format!("{:x}", hash.finalize()))
    }
}
fn open(path: &Path, parent: Option<&File>, directory: bool, symlink: bool) -> Result<File> {
    let flags = rustix::fs::OFlags::RDONLY
        | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::NONBLOCK
        | rustix::fs::OFlags::CLOEXEC;
    let flags = if directory {
        flags | rustix::fs::OFlags::DIRECTORY
    } else {
        flags
    };
    let flags = if symlink {
        // Darwin O_SYMLINK opens the link itself; combining O_NOFOLLOW returns
        // ELOOP. The opened link's full stamp is checked before xattr reads.
        flags.difference(rustix::fs::OFlags::NOFOLLOW) | rustix::fs::OFlags::SYMLINK
    } else {
        flags
    };
    if let Some(parent) = parent {
        rustix::fs::openat(
            parent,
            path.file_name().ok_or_else(invalid)?,
            flags,
            rustix::fs::Mode::empty(),
        )
        .map(File::from)
        .map_err(|_| invalid())
    } else {
        OpenOptions::new()
            .read(true)
            .custom_flags(flags.bits() as i32)
            .open(path)
            .map_err(|_| invalid())
    }
}
fn field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value);
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_BUNDLE_CHANGED",
        "Application tree is unsupported or changed during verification; no copy was accepted.",
        3,
    )
}

#[cfg(test)]
mod cache_tests;

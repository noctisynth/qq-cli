//! Permission changes require the live capability returned by verified copying.
use super::*;

fn mode(entry: &Entry) -> u32 {
    match entry.content {
        Content::Directory => 0o700,
        Content::File { .. } => 0o600 | (entry.mode & 0o100),
        Content::Link(_) => entry.mode,
    }
}

impl Inventory {
    /// Consume a verified private copy, remove its ACLs and normalize owner-only
    /// access. Failure retains a partially changed tree, never a usable receipt.
    /// Merely inspecting a path cannot create the required ownership capability.
    pub fn harden_copy(
        mut self,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let owner = self.owner.as_ref().ok_or_else(invalid)?;
        let check = || {
            owner.verify()?;
            guard()?;
            owner.verify()
        };
        check()?;
        self.revalidate(deadline)?;
        let root = open(&self.root, None, true, false)?;
        self.harden_node(Path::new(""), &root, deadline, &check)?;
        check()?;
        let mut result = Self::inspect(&self.root, deadline)?;
        if result.entries.len() != self.entries.len() {
            return Err(invalid());
        }
        for (path, expected) in &self.entries {
            let actual = result.entries.get(path).ok_or_else(invalid)?;
            let before = &self.stamps[path];
            let after = &result.stamps[path];
            if actual.mode != mode(expected)
                || actual.acl != crate::native::macos_acl::absent_digest()
                || actual.content != expected.content
                || actual.attributes != expected.attributes
                || actual.flags != expected.flags
                || before.device != after.device
                || before.inode != after.inode
                || before.uid != after.uid
                || before.gid != after.gid
                || before.links != after.links
                || before.size != after.size
                || before.modified != after.modified
                || after.uid != rustix::process::geteuid().as_raw()
            {
                return Err(invalid());
            }
        }
        check()?;
        result.verify_stamps(deadline)?;
        result.owner = self.owner.take();
        Ok(result)
    }

    fn harden_node(
        &self,
        relative: &Path,
        file: &File,
        deadline: Instant,
        guard: &impl Fn() -> Result<()>,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        let expected = self.entries.get(relative).ok_or_else(invalid)?;
        let stamp = self.stamps.get(relative).ok_or_else(invalid)?;
        if Stamp::from(&file.metadata().map_err(|_| invalid())?) != *stamp
            || stamp.uid != rustix::process::geteuid().as_raw()
        {
            return Err(invalid());
        }
        if matches!(expected.content, Content::Directory) {
            for child in rustix::fs::Dir::read_from(file).map_err(|_| invalid())? {
                let child = child.map_err(|_| invalid())?;
                let name = child.file_name().to_bytes();
                if name == b"." || name == b".." {
                    continue;
                }
                let path = relative.join(std::ffi::OsStr::from_bytes(name));
                let entry = self.entries.get(&path).ok_or_else(invalid)?;
                let held = open(
                    &path,
                    Some(file),
                    matches!(entry.content, Content::Directory),
                    matches!(entry.content, Content::Link(_)),
                )?;
                self.harden_node(&path, &held, deadline, guard)?;
            }
        }
        guard()?;
        if Stamp::from(&file.metadata().map_err(|_| invalid())?) != *stamp {
            return Err(invalid());
        }
        crate::native::macos_acl::clear_owned(file.as_fd())?;
        guard()?;
        if !matches!(expected.content, Content::Link(_)) {
            rustix::fs::fchmod(
                file,
                rustix::fs::Mode::from_bits_truncate(mode(expected) as u16),
            )
            .map_err(|_| invalid())?;
            if matches!(expected.content, Content::Directory) {
                file.sync_all().map_err(|_| invalid())?;
            } else {
                rustix::fs::fcntl_fullfsync(file).map_err(|_| invalid())?;
            }
        }
        let current = std::fs::symlink_metadata(self.root.join(relative)).map_err(|_| invalid())?;
        if current.dev() != stamp.device
            || current.ino() != stamp.inode
            || current.uid() != stamp.uid
            || current.mode() & 0o777 != mode(expected)
        {
            return Err(invalid());
        }
        guard()?;
        crate::core::deadline::check(deadline)
    }
}

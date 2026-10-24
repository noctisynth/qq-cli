//! Metadata editing on a live hardened private copy, without signing/startup.
use super::*;
use std::io::Write;

const PACKAGE: &str = "Contents/Resources/app/package.json";

impl Inventory {
    pub(crate) fn package_bytes(&self, deadline: Instant) -> Result<Vec<u8>> {
        self.verify_stamps(deadline)?;
        let path = Path::new(PACKAGE);
        let entry = self.entries.get(path).ok_or_else(invalid)?;
        let Content::File { size, hash } = &entry.content else {
            return Err(invalid());
        };
        if *size > 1024 * 1024 {
            return Err(invalid());
        }
        let mut file = self.signing_file(path)?;
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        let actual: [u8; 32] = Sha256::digest(&bytes).into();
        if bytes.len() as u64 != *size
            || actual != *hash
            || Stamp::from(&file.metadata().map_err(|_| invalid())?) != self.stamps[path]
        {
            return Err(invalid());
        }
        self.verify_stamps(deadline)?;
        Ok(bytes)
    }

    pub(crate) fn deploy_package(
        mut self,
        destination: &Path,
        source_digest: &str,
        bytes: &[u8],
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let owner = self.owner.take().ok_or_else(invalid)?;
        if destination != self.root.join(PACKAGE) || bytes.len() > 2 * 1024 * 1024 {
            return Err(invalid());
        }
        for (path, entry) in &self.entries {
            let expected = match entry.content {
                Content::Directory => 0o700,
                Content::File { .. } => 0o600 | (entry.mode & 0o100),
                Content::Link(_) => entry.mode,
            };
            if entry.mode != expected
                || entry.acl != crate::native::macos_acl::absent_digest()
                || self.stamps[path].uid != rustix::process::geteuid().as_raw()
            {
                return Err(invalid());
            }
        }
        let path = Path::new(PACKAGE);
        let entry = self.entries.get(path).ok_or_else(invalid)?;
        match &entry.content {
            Content::File { size, hash } if *size <= 1024 * 1024 => {
                let digest: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
                if digest != source_digest {
                    return Err(invalid());
                }
            }
            _ => return Err(invalid()),
        }
        let compressed = entry.flags & libc::UF_COMPRESSED != 0;
        let check = || {
            crate::core::deadline::check(deadline)?;
            owner.verify()?;
            guard()?;
            owner.verify()
        };
        check()?;
        self.revalidate(deadline)?;
        let mut held = vec![open(&self.root, None, true, false)?];
        let mut relative = PathBuf::new();
        for name in ["Contents", "Resources", "app"] {
            relative.push(name);
            let file = open(&relative, held.last(), true, false)?;
            if Stamp::from(&file.metadata().map_err(|_| invalid())?) != self.stamps[&relative] {
                return Err(invalid());
            }
            held.push(file);
        }
        // Writable open itself can decompress. Check authority and the original
        // tree BEFORE that effect, including the held read-only package inode.
        let original = open(path, held.last(), false, false)?;
        check()?;
        self.verify_stamps(deadline)?;
        if Stamp::from(&original.metadata().map_err(|_| invalid())?) != self.stamps[path] {
            return Err(invalid());
        }
        let mut file = File::from(
            rustix::fs::openat(
                held.last().ok_or_else(invalid)?,
                "package.json",
                rustix::fs::OFlags::WRONLY
                    | rustix::fs::OFlags::NOFOLLOW
                    | rustix::fs::OFlags::NONBLOCK
                    | rustix::fs::OFlags::CLOEXEC,
                rustix::fs::Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        if compressed {
            rustix::fs::fcntl_fullfsync(&file).map_err(|_| invalid())?;
            self = self.verify_decompressed_package(deadline)?;
        }
        // Opening may block even for uncompressed files. Recheck the deadline
        // and lifecycle authority before the next effect (the metadata write).
        check()?;
        self.verify_stamps(deadline)?;
        if Stamp::from(&file.metadata().map_err(|_| invalid())?) != self.stamps[path] {
            return Err(invalid());
        }
        // No truncation at open or path-based replacement. Failure consumes
        // the capability and leaves the partial copy for lifecycle recovery.
        file.write_all(bytes).map_err(|_| invalid())?;
        file.set_len(bytes.len() as u64).map_err(|_| invalid())?;
        rustix::fs::fcntl_fullfsync(&file).map_err(|_| invalid())?;
        check()?;
        let mut result = Self::inspect(&self.root, deadline)?;
        if result.entries.len() != self.entries.len() {
            return Err(invalid());
        }
        for (name, before) in &self.entries {
            let after = result.entries.get(name).ok_or_else(invalid)?;
            if name != path {
                if before != after || self.stamps[name] != result.stamps[name] {
                    return Err(invalid());
                }
                continue;
            }
            let old = &self.stamps[name];
            let new = &result.stamps[name];
            if after.content
                != (Content::File {
                    size: bytes.len() as u64,
                    hash: Sha256::digest(bytes).into(),
                })
                || after.mode != before.mode
                || after.acl != before.acl
                || after.flags != before.flags
                || after.attributes != before.attributes
                || old.device != new.device
                || old.inode != new.inode
                || old.uid != new.uid
                || old.gid != new.gid
                || old.links != new.links
            {
                return Err(invalid());
            }
        }
        check()?;
        result.verify_stamps(deadline)?;
        result.owner = Some(owner);
        Ok(result)
    }

    fn verify_decompressed_package(&self, deadline: Instant) -> Result<Self> {
        let result = Self::inspect(&self.root, deadline)?;
        if result.entries.len() != self.entries.len() {
            return Err(invalid());
        }
        for (path, before) in &self.entries {
            let after = result.entries.get(path).ok_or_else(invalid)?;
            let old = &self.stamps[path];
            let new = &result.stamps[path];
            if path != Path::new(PACKAGE) {
                if before != after || old != new {
                    return Err(invalid());
                }
            } else if before.content != after.content
                || before.mode != after.mode
                || before.acl != after.acl
                || after.flags != before.flags & !libc::UF_COMPRESSED
                || !before.attributes.verifies_decompression(&after.attributes)
                || old.device != new.device
                || old.inode != new.inode
                || old.uid != new.uid
                || old.gid != new.gid
                || old.links != new.links
            {
                return Err(invalid());
            }
        }
        Ok(result)
    }
}

//! Checked private staging of an unmodified application, before entry edits/signing.
use super::*;
use crate::native::macos_copy_worker::Worker;
use rustix::fs::{Mode, OFlags};

pub(super) struct Parent {
    path: PathBuf,
    file: File,
}
impl Parent {
    pub(super) fn verify(&self) -> Result<()> {
        reject_links(&self.path)?;
        let current = std::fs::symlink_metadata(&self.path).map_err(|_| invalid())?;
        let original = self.file.metadata().map_err(|_| invalid())?;
        if !current.is_dir()
            || current.dev() != original.dev()
            || current.ino() != original.ino()
            || current.uid() != rustix::process::geteuid().as_raw()
            || current.mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        crate::native::macos_acl::require_absent(self.file.as_fd())
    }
}

impl Inventory {
    /// Preserve and verify a new copy under an existing private parent.
    /// No source authentication, permission hardening, signing or execution.
    /// On failure, staging (or a published-but-unconfirmed copy) is retained.
    pub fn copy_to_new(
        &self,
        parent: &Path,
        name: &str,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        self.copy_new(parent, name, deadline, guard, &mut None)
    }

    /// Production host must register the private worker entry. Native copying is
    /// isolated; validation, quarantine preservation, sync and publication remain
    /// in this parent. These filesystem operations have cooperative deadlines.
    pub fn copy_to_new_isolated(
        &self,
        parent: &Path,
        name: &str,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        let mut worker = Some(Worker::start(deadline)?);
        self.copy_using_worker(parent, name, deadline, guard, &mut worker)
    }

    fn copy_using_worker(
        &self,
        parent: &Path,
        name: &str,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
        worker: &mut Option<Worker>,
    ) -> Result<Self> {
        self.copy_new(parent, name, deadline, guard, worker)
            .map_err(|error| {
                if let Some(worker) = worker.as_mut() {
                    worker.abort(error)
                } else {
                    error
                }
            })
    }

    fn copy_new(
        &self,
        parent: &Path,
        name: &str,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
        worker: &mut Option<Worker>,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        if !parent.is_absolute()
            || parent.canonicalize().map_err(|_| invalid())? != parent
            || name.is_empty()
            || name.contains(['/', '\\'])
            || name.chars().any(char::is_control)
            || !name.ends_with(".app")
            || name.len() > 255
            || parent.starts_with(&self.root)
            || self.root.starts_with(parent)
        {
            return Err(invalid());
        }
        // Unknown/system or immutable/append flags need a separate deployment
        // policy. Never create a staged tree that cannot be completed/cleaned.
        let allowed = libc::UF_COMPRESSED | libc::UF_HIDDEN | libc::UF_NODUMP;
        if self
            .entries
            .values()
            .any(|entry| entry.flags & !allowed != 0)
        {
            return Err(invalid());
        }
        reject_links(parent)?;
        let parent = Parent {
            path: parent.into(),
            file: open(parent, None, true, false)?,
        };
        parent.verify()?;
        guard()?;
        self.revalidate(deadline)?;
        parent.verify()?;
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|_| invalid())?;
        let staging = format!(
            ".qq-cli-copy-{:032x}.partial.app",
            u128::from_le_bytes(random)
        );
        rustix::fs::mkdirat(&parent.file, staging.as_str(), Mode::RWXU).map_err(|_| invalid())?;
        let stage_path = parent.path.join(&staging);
        let source = open(&self.root, None, true, false)?;
        let destination = open(&stage_path, Some(&parent.file), true, false)?;
        let check = || {
            parent.verify()?;
            guard()?;
            parent.verify()
        };
        self.copy_node(
            Path::new(""),
            &source,
            &destination,
            deadline,
            &check,
            worker,
        )?;
        if let Some(worker) = worker.take() {
            worker.finish(check)?;
        }
        check()?;
        let candidate = self.verify_copy(&stage_path, deadline)?;
        let held = destination.metadata().map_err(|_| invalid())?;
        let staged = &candidate.stamps[Path::new("")];
        if staged.device != held.dev() || staged.inode != held.ino() {
            return Err(invalid());
        }
        self.revalidate(deadline)?;
        candidate.revalidate(deadline)?;
        check()?;
        rustix::fs::renameat_with(
            &parent.file,
            staging.as_str(),
            &parent.file,
            name,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(|_| invalid())?;
        parent.file.sync_all().map_err(|_| invalid())?;
        check()?;
        let mut published = Self::inspect(&parent.path.join(name), deadline)?;
        if published.entries != candidate.entries {
            return Err(invalid());
        }
        let before = &candidate.stamps[Path::new("")];
        let after = &published.stamps[Path::new("")];
        if before.device != after.device || before.inode != after.inode {
            return Err(invalid());
        }
        for (path, stamp) in &published.stamps {
            if stamp.uid != rustix::process::geteuid().as_raw()
                || (!path.as_os_str().is_empty() && candidate.stamps.get(path) != Some(stamp))
            {
                return Err(invalid());
            }
        }
        self.verify_stamps(deadline)?;
        check()?;
        published.owner = Some(parent);
        Ok(published)
    }

    fn copy_node(
        &self,
        relative: &Path,
        source: &File,
        destination: &File,
        deadline: Instant,
        guard: &impl Fn() -> Result<()>,
        worker: &mut Option<Worker>,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        let entry = self.entries.get(relative).ok_or_else(invalid)?;
        let expected = self.stamps.get(relative).ok_or_else(invalid)?;
        if Stamp::from(&source.metadata().map_err(|_| invalid())?) != *expected {
            return Err(invalid());
        }
        if matches!(entry.content, Content::Directory) {
            for child in rustix::fs::Dir::read_from(source).map_err(|_| invalid())? {
                let child = child.map_err(|_| invalid())?;
                let bytes = child.file_name().to_bytes();
                if bytes == b"." || bytes == b".." {
                    continue;
                }
                let name = std::ffi::OsStr::from_bytes(bytes);
                let path = relative.join(name);
                let item = self.entries.get(&path).ok_or_else(invalid)?;
                let is_dir = matches!(item.content, Content::Directory);
                let is_link = matches!(item.content, Content::Link(_));
                let input = open(&self.root.join(&path), Some(source), is_dir, is_link)?;
                let output = match &item.content {
                    Content::Directory => {
                        rustix::fs::mkdirat(destination, name, Mode::RWXU)
                            .map_err(|_| invalid())?;
                        open(&path, Some(destination), true, false)?
                    }
                    Content::Link(target) => {
                        rustix::fs::symlinkat(target, destination, name).map_err(|_| invalid())?;
                        open(&path, Some(destination), false, true)?
                    }
                    Content::File { .. } => rustix::fs::openat(
                        destination,
                        name,
                        OFlags::RDWR
                            | OFlags::CREATE
                            | OFlags::EXCL
                            | OFlags::NOFOLLOW
                            | OFlags::CLOEXEC,
                        Mode::RUSR | Mode::WUSR,
                    )
                    .map(File::from)
                    .map_err(|_| invalid())?,
                };
                self.copy_node(&path, &input, &output, deadline, guard, worker)?;
            }
        }
        let data = matches!(entry.content, Content::File { .. });
        if let Some(worker) = worker.as_mut() {
            worker.copy(source.as_fd(), destination.as_fd(), data, guard)?;
        } else {
            crate::native::macos_copy::node(source.as_fd(), destination.as_fd(), data, deadline)?;
        }
        entry.attributes.copy_quarantine(source, destination)?;
        if matches!(entry.content, Content::File { .. }) {
            rustix::fs::fcntl_fullfsync(destination).map_err(|_| invalid())?;
        } else if matches!(entry.content, Content::Directory) {
            destination.sync_all().map_err(|_| invalid())?;
        }
        if Stamp::from(&source.metadata().map_err(|_| invalid())?) != *expected {
            return Err(invalid());
        }
        guard()?;
        crate::core::deadline::check(deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    use std::time::Duration;

    #[test]
    fn isolated_tree_finishes_worker_before_verified_publication() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let source = base.join("Source.app");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("data"), b"held tree").unwrap();
        std::os::unix::fs::symlink("data", source.join("alias")).unwrap();
        let parent = base.join("private");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&parent)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let original = Inventory::inspect(&source, deadline).unwrap();
        let mut worker = Some(crate::native::macos_copy_worker::tests::fixture(
            "live", 10000, deadline,
        ));
        let copy = original
            .copy_using_worker(&parent, "QQ.app", deadline, || Ok(()), &mut worker)
            .unwrap();
        assert!(worker.is_none());
        assert_eq!(copy.fingerprint(), original.fingerprint());
        copy.revalidate(deadline).unwrap();
        let hardened = copy.harden_copy(deadline, || Ok(())).unwrap();
        hardened.revalidate(deadline).unwrap();
        original.revalidate(deadline).unwrap();
    }

    #[test]
    fn isolated_tree_timeout_never_publishes() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let source = base.join("Source.app");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("data"), b"held tree").unwrap();
        let parent = base.join("private");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&parent)
            .unwrap();
        let deadline = Instant::now() + Duration::from_millis(100);
        let original = Inventory::inspect(&source, deadline).unwrap();
        let mut worker = Some(crate::native::macos_copy_worker::tests::fixture(
            "ignore", 10000, deadline,
        ));
        assert_eq!(
            original
                .copy_using_worker(&parent, "QQ.app", deadline, || Ok(()), &mut worker)
                .err()
                .unwrap()
                .code,
            "DEADLINE"
        );
        assert!(!parent.join("QQ.app").exists());
    }
}

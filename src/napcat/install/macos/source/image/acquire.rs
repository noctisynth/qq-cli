//! Fixed-origin acquisition and bounded no-overwrite cache publication.
use super::*;
use crate::storage::state::{Store, durable::Lock, private_dir};
use std::{
    io::{Read, Write},
    os::{fd::AsFd, macos::fs::MetadataExt as _},
};
mod http;

pub fn acquire(store: &Store, from: Option<&Path>, deadline: Instant) -> Result<Image> {
    acquire_with(
        store,
        super::super::DMG_BYTES,
        super::super::DMG_SHA256,
        deadline,
        |destination| {
            if let Some(path) = from {
                let image = Image::inspect(path, deadline)?;
                quarantine::copy(&image, destination, deadline)?;
                Ok(Box::new(LocalInput { image, deadline }) as Box<dyn Read>)
            } else {
                Ok(Box::new(http::download(deadline)?) as Box<dyn Read>)
            }
        },
    )
}
pub(super) fn directory(path: &Path) -> Result<File> {
    reject_links(path)?;
    let file = File::from(
        rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| invalid())?,
    );
    let m = file.metadata().map_err(|_| invalid())?;
    if m.uid() != rustix::process::geteuid().as_raw()
        || m.mode() & 0o7777 != 0o700
        || m.st_flags() != 0
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(file.as_fd())?;
    Ok(file)
}
fn same(a: &File, b: &File) -> Result<bool> {
    let a = a.metadata().map_err(|_| invalid())?;
    let b = b.metadata().map_err(|_| invalid())?;
    Ok(a.dev() == b.dev() && a.ino() == b.ino())
}
pub(super) fn private_file(file: &File) -> Result<()> {
    let m = file.metadata().map_err(|_| invalid())?;
    if !m.is_file()
        || m.nlink() != 1
        || m.uid() != rustix::process::geteuid().as_raw()
        || m.mode() & 0o7777 != 0o600
        || m.st_flags() != 0
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(file.as_fd())
}
fn acquire_with(
    store: &Store,
    bytes: u64,
    digest: &str,
    deadline: Instant,
    source: impl FnOnce(&File) -> Result<Box<dyn Read>>,
) -> Result<Image> {
    crate::core::deadline::check(deadline)?;
    private_dir(store.root())?;
    if store.root().canonicalize().map_err(|_| invalid())? != store.root() {
        return Err(invalid());
    }
    let lock = Lock::acquire(store.root(), "qq-image.lock", deadline, "QQ_IMAGE_BUSY")?;
    let root = store.root().join("qq-images");
    private_dir(&root)?;
    let parent = directory(&root)?;
    let guard = || {
        crate::core::deadline::check(deadline)?;
        lock.verify()?;
        if !same(&parent, &directory(&root)?)? {
            return Err(invalid());
        }
        Ok(())
    };
    let name = format!("{digest}.dmg");
    let target = root.join(&name);
    reject_links(&target)?;
    if target.try_exists().map_err(|_| invalid())? {
        let image = Image::inspect_expected(&target, bytes, digest, deadline)?;
        private_file(&image.file)?;
        guard()?;
        return Ok(image);
    }
    let stage_name = format!("{digest}.partial");
    let stage = root.join(&stage_name);
    reject_links(&stage)?;
    // Only complete pinned bytes can be recovered. An incomplete or changed
    // residue is never appended to, truncated, deleted or replaced here.
    if stage.try_exists().map_err(|_| invalid())? {
        let staged = Image::inspect_expected(&stage, bytes, digest, deadline)?;
        private_file(&staged.file)?;
        guard()?;
        let file = File::from(
            rustix::fs::openat(
                &parent,
                stage_name.as_str(),
                OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        if !same(&file, &staged.file)? {
            return Err(invalid());
        }
        return publish(&parent, &file, &staged, &target, deadline, &guard);
    }
    guard()?;
    let mut file = File::from(
        rustix::fs::openat(
            &parent,
            stage_name.as_str(),
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(|_| invalid())?,
    );
    let original = file.try_clone().map_err(|_| invalid())?;
    let work = (|| {
        private_file(&file)?;
        guard()?;
        let mut input = source(&file)?;
        // Persist source quarantine before any bytes can form a recoverable image.
        rustix::fs::fcntl_fullfsync(&file).map_err(|_| invalid())?;
        let mut offset = 0u64;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            guard()?;
            if !same(&file, &open(&stage)?)? {
                return Err(invalid());
            }
            let limit = ((bytes - offset).saturating_add(1)).min(buffer.len() as u64) as usize;
            let count = input.read(&mut buffer[..limit]).map_err(|_| invalid())?;
            guard()?;
            if count == 0 {
                break;
            }
            if count as u64 > bytes - offset {
                return Err(invalid());
            }
            if !same(&file, &open(&stage)?)? {
                return Err(invalid());
            }
            private_file(&file)?;
            file.write_all(&buffer[..count]).map_err(|_| invalid())?;
            hash.update(&buffer[..count]);
            offset += count as u64;
        }
        if offset != bytes || format!("{:x}", hash.finalize()) != digest {
            return Err(invalid());
        }
        rustix::fs::fcntl_fullfsync(&file).map_err(|_| invalid())?;
        let staged = Image::inspect_expected(&stage, bytes, digest, deadline)?;
        if !same(&file, &staged.file)? {
            return Err(invalid());
        }
        publish(&parent, &file, &staged, &target, deadline, &guard)
    })();
    // Only this invocation's still-identical staging file may be removed. An
    // unknown path, lost lock or already published destination is preserved.
    if work.is_err()
        && guard().is_ok()
        && let Ok(current) = open(&stage)
        && same(&original, &current).unwrap_or(false)
    {
        let _ = rustix::fs::unlinkat(&parent, stage_name.as_str(), rustix::fs::AtFlags::empty());
        let _ = parent.sync_all();
    }
    work
}
fn publish(
    parent: &File,
    file: &File,
    staged: &Image,
    target: &Path,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Image> {
    guard()?;
    private_file(file)?;
    staged.revalidate(deadline)?;
    if !same(file, &staged.file)? {
        return Err(invalid());
    }
    rustix::fs::fcntl_fullfsync(file).map_err(|_| invalid())?;
    guard()?;
    if !same(file, &open(staged.path())?)? {
        return Err(invalid());
    }
    rustix::fs::renameat_with(
        parent,
        staged.path().file_name().ok_or_else(invalid)?,
        parent,
        target.file_name().ok_or_else(invalid)?,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|_| invalid())?;
    parent.sync_all().map_err(|_| invalid())?;
    rustix::fs::fcntl_fullfsync(file).map_err(|_| invalid())?;
    guard()?;
    let image = Image::inspect_expected(target, staged.stamp.size, &staged.digest, deadline)?;
    if !same(file, &image.file)? || image.quarantine != staged.quarantine {
        return Err(invalid());
    }
    private_file(&image.file)?;
    Ok(image)
}

#[cfg(test)]
mod tests;

struct LocalInput {
    image: Image,
    deadline: Instant,
}
impl Read for LocalInput {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let count = self.image.file.read(buffer)?;
        if count == 0 {
            self.image
                .revalidate(self.deadline)
                .map_err(|_| std::io::Error::other("disk image source changed"))?;
        }
        Ok(count)
    }
}

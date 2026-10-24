//! Existing read-only mount observation. Never authorizes attach or detach.
use super::{Image, open};
use crate::core::error::{AppError, Result};
use crate::storage::state::reject_links;
use std::{
    fs::File,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Instant,
};
pub(super) mod command;
mod devices;
mod media;
pub use devices::Devices;
pub(super) use devices::Transfer;
mod report;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    selected: report::Selected,
    media: media::Generation,
    device: u64,
    inode: u64,
}
impl Receipt {
    pub(super) fn require_gone(
        &self,
        image: &Image,
        mount: &Path,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<()> {
        require_absent(image.path(), mount, deadline, guard)?;
        media::require_gone(&self.media, deadline, guard)
    }
    pub(super) fn pin(
        &self,
        image: &Image,
        mount: &Path,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<File> {
        let observed = Observation::inspect(image, mount, deadline, guard)?;
        if observed.receipt()? != *self {
            return Err(invalid());
        }
        Ok(observed.root)
    }
}
pub(super) fn require_absent(
    image: &Path,
    mount: &Path,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    report::absent(&command::info(deadline, guard)?, image, mount)
}

pub struct Observation<'a> {
    image: &'a Image,
    selected: report::Selected,
    root: File,
    media: media::Generation,
}
impl<'a> Observation<'a> {
    pub(super) fn receipt(&self) -> Result<Receipt> {
        let metadata = self.root.metadata().map_err(|_| invalid())?;
        Ok(Receipt {
            selected: self.selected.clone(),
            media: self.media.clone(),
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }
    pub fn inspect(
        image: &'a Image,
        mount: &Path,
        deadline: Instant,
        guard: &dyn Fn() -> Result<()>,
    ) -> Result<Self> {
        guard()?;
        image.revalidate(deadline)?;
        let selected = report::select(&command::info(deadline, guard)?, image.path(), mount)?;
        let root = open_root(mount)?;
        verify_filesystem(&root, &selected)?;
        let media = media::observe(&selected, deadline, guard)?;
        let observed = Self {
            image,
            selected,
            root,
            media,
        };
        observed.revalidate(deadline, guard)?;
        Ok(observed)
    }
    pub fn root(&self) -> &Path {
        &self.selected.mount
    }

    pub fn revalidate(&self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        guard()?;
        self.image.revalidate(deadline)?;
        let current = report::select(
            &command::info(deadline, guard)?,
            self.image.path(),
            self.root(),
        )?;
        if current != self.selected {
            return Err(invalid());
        }
        if media::observe(&current, deadline, guard)? != self.media {
            return Err(invalid());
        }
        verify_filesystem(&self.root, &self.selected)?;
        let file = open_root(self.root())?;
        verify_filesystem(&file, &self.selected)?;
        let held = self.root.metadata().map_err(|_| invalid())?;
        let now = file.metadata().map_err(|_| invalid())?;
        if held.dev() != now.dev() || held.ino() != now.ino() {
            return Err(invalid());
        }
        guard()?;
        crate::core::deadline::check(deadline)
    }
}
fn open_root(path: &Path) -> Result<File> {
    reject_links(path)?;
    let file = open(path)?;
    if !file.metadata().map_err(|_| invalid())?.is_dir() {
        return Err(invalid());
    }
    Ok(file)
}
fn name(bytes: &[libc::c_char]) -> Result<Vec<u8>> {
    let end = bytes.iter().position(|v| *v == 0).ok_or_else(invalid)?;
    Ok(bytes[..end].iter().map(|v| *v as u8).collect())
}
fn verify_filesystem(file: &File, selected: &report::Selected) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let fs = rustix::fs::fstatfs(file).map_err(|_| invalid())?;
    if fs.f_flags & libc::MNT_RDONLY as u32 == 0
        || name(&fs.f_mntfromname)? != selected.device.as_bytes()
        || name(&fs.f_mntonname)? != selected.mount.as_os_str().as_bytes()
    {
        return Err(invalid());
    }
    Ok(())
}
fn invalid() -> AppError {
    AppError::new(
        "QQ_MOUNT_INVALID",
        "Read-only image mount identity could not be verified; no mount ownership was inferred.",
        4,
    )
}

#[cfg(test)]
mod tests;

//! Private fixed-version configuration publication. The caller must persist its
//! credential/preparation intent before entering this low-level effect kernel.
//! No Keychain access, instance promotion, launch permission or repair here.
use super::{macos_layout::Layout, runtime_config::Bundle};
use crate::core::error::{AppError, Result};
use rustix::fs::{Mode, OFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    os::{fd::AsFd, macos::fs::MetadataExt as _, unix::fs::MetadataExt},
    time::Instant,
};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Identity {
    device: u64,
    inode: u64,
    length: u64,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Published {
    layout: String,
    files: BTreeMap<String, (Identity, String)>,
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_CONFIG_CONFLICT",
        "Private runtime configuration changed or could not be published; existing material was preserved.",
        4,
    )
}
fn flags() -> OFlags {
    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK
}
fn identity(file: &File, directory: bool) -> Result<Identity> {
    let meta = file.metadata().map_err(|_| invalid())?;
    if meta.uid() != rustix::process::geteuid().as_raw()
        || meta.st_flags() != 0
        || (if directory {
            !meta.is_dir() || meta.mode() & 0o7777 != 0o700
        } else {
            !meta.is_file()
                || meta.nlink() != 1
                || meta.mode() & 0o7777 != 0o600
                || meta.len() > 16384
        })
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(file.as_fd())?;
    Ok(Identity {
        device: meta.dev(),
        inode: meta.ino(),
        length: if directory { 0 } else { meta.len() },
    })
}
fn directory(layout: &Layout, deadline: Instant) -> Result<File> {
    layout.verify(deadline)?;
    let file = File::from(
        rustix::fs::open(
            layout.directory().join("napcat-data/config"),
            flags() | OFlags::DIRECTORY,
            Mode::empty(),
        )
        .map_err(|_| invalid())?,
    );
    identity(&file, true)?;
    layout.verify(deadline)?;
    Ok(file)
}
fn check_parent(layout: &Layout, held: &File, deadline: Instant) -> Result<()> {
    if identity(held, true)? != identity(&directory(layout, deadline)?, true)? {
        return Err(invalid());
    }
    Ok(())
}
fn names(directory: &File) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in rustix::fs::Dir::read_from(directory).map_err(|_| invalid())? {
        let entry = entry.map_err(|_| invalid())?;
        let name = entry.file_name().to_bytes();
        if name == b"." || name == b".." {
            continue;
        }
        if names.len() == 3 {
            return Err(invalid());
        }
        names.push(std::str::from_utf8(name).map_err(|_| invalid())?.to_owned());
    }
    names.sort();
    Ok(names)
}
pub(crate) fn require_empty(layout: &Layout, deadline: Instant) -> Result<()> {
    let held = directory(layout, deadline)?;
    if !names(&held)?.is_empty() {
        return Err(invalid());
    }
    check_parent(layout, &held, deadline)
}

fn render(bundle: &Bundle) -> Result<BTreeMap<String, Vec<u8>>> {
    if bundle.version() != crate::napcat::install::catalog::lookup("4.18.41")?.version {
        return Err(invalid());
    }
    let mut files = BTreeMap::new();
    for file in bundle.files() {
        let mut bytes = Vec::new();
        file.write_to(&mut bytes)?;
        if bytes.len() > 16384 {
            return Err(invalid());
        }
        files.insert(file.name().to_owned(), bytes);
    }
    if files.keys().map(String::as_str).collect::<Vec<_>>()
        != ["napcat.json", "onebot11.json", "webui.json"]
    {
        return Err(invalid());
    }
    Ok(files)
}
impl Published {
    /// Before first startup only. Account/runtime-mutated files need a separate
    /// observation policy; this method never repairs or rewrites them.
    pub fn verify(&self, layout: &Layout, bundle: &Bundle, deadline: Instant) -> Result<()> {
        if self.layout != layout.fingerprint() {
            return Err(invalid());
        }
        let expected = render(bundle)?;
        let directory = directory(layout, deadline)?;
        if names(&directory)? != expected.keys().cloned().collect::<Vec<_>>()
            || self.files.keys().collect::<Vec<_>>() != expected.keys().collect::<Vec<_>>()
        {
            return Err(invalid());
        }
        for (name, bytes) in expected {
            check_parent(layout, &directory, deadline)?;
            let mut file = File::from(
                rustix::fs::openat(&directory, name.as_str(), flags(), Mode::empty())
                    .map_err(|_| invalid())?,
            );
            let original = identity(&file, false)?;
            let (recorded, digest) = self.files.get(&name).ok_or_else(invalid)?;
            if original != *recorded || digest != &format!("{:x}", Sha256::digest(&bytes)) {
                return Err(invalid());
            }
            let mut actual = Vec::new();
            (&mut file)
                .take(16385)
                .read_to_end(&mut actual)
                .map_err(|_| invalid())?;
            if actual != bytes || identity(&file, false)? != original {
                return Err(invalid());
            }
            let current = File::from(
                rustix::fs::openat(&directory, name.as_str(), flags(), Mode::empty())
                    .map_err(|_| invalid())?,
            );
            if identity(&current, false)? != original {
                return Err(invalid());
            }
        }
        check_parent(layout, &directory, deadline)
    }
}
/// Caller owns the lifecycle intent and guard. Failure retains partial files;
/// an existing name, account file or extra content is never adopted/overwritten.
pub fn publish(
    layout: &Layout,
    bundle: &Bundle,
    deadline: Instant,
    guard: impl Fn() -> Result<()>,
) -> Result<Published> {
    guard()?;
    let files = render(bundle)?;
    let directory = directory(layout, deadline)?;
    if !names(&directory)?.is_empty() {
        return Err(invalid());
    }
    let mut published = Published {
        layout: layout.fingerprint().into(),
        files: BTreeMap::new(),
    };
    for (name, bytes) in files {
        guard()?;
        check_parent(layout, &directory, deadline)?;
        let mut file = File::from(
            rustix::fs::openat(
                &directory,
                name.as_str(),
                OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(|_| invalid())?,
        );
        let created = identity(&file, false)?;
        if created.device != identity(&directory, true)?.device {
            return Err(invalid());
        }
        guard()?;
        check_parent(layout, &directory, deadline)?;
        let current = File::from(
            rustix::fs::openat(&directory, name.as_str(), flags(), Mode::empty())
                .map_err(|_| invalid())?,
        );
        if identity(&current, false)? != created {
            return Err(invalid());
        }
        file.write_all(&bytes).map_err(|_| invalid())?;
        guard()?;
        check_parent(layout, &directory, deadline)?;
        rustix::fs::fcntl_fullfsync(&file).map_err(|_| invalid())?;
        published.files.insert(
            name,
            (
                identity(&file, false)?,
                format!("{:x}", Sha256::digest(bytes)),
            ),
        );
    }
    guard()?;
    published.verify(layout, bundle, deadline)?;
    directory.sync_all().map_err(|_| invalid())?;
    guard()?;
    published.verify(layout, bundle, deadline)?;
    Ok(published)
}

#[cfg(test)]
mod tests;

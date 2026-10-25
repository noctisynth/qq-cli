//! Permission checks on retained descriptors; never repairs user ACLs or modes.
use super::*;
use std::os::fd::AsFd;

pub(super) fn directory(file: &File) -> Result<()> {
    let meta = file.metadata().map_err(|_| AppError::storage())?;
    if !meta.is_dir()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o022 != 0
    {
        return Err(AppError::storage());
    }
    no_acl(file)
}

pub(super) fn protected_file(file: &File) -> Result<()> {
    regular(file, 0o022)
}

pub(super) fn private_file(file: &File) -> Result<()> {
    regular(file, 0o077)
}

fn regular(file: &File, forbidden: u32) -> Result<()> {
    let meta = file.metadata().map_err(|_| AppError::storage())?;
    if !meta.is_file()
        || meta.nlink() != 1
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & forbidden != 0
    {
        return Err(AppError::storage());
    }
    no_acl(file)
}

fn no_acl(file: &File) -> Result<()> {
    crate::native::macos_acl::require_absent(file.as_fd()).map_err(|_| AppError::storage())
}

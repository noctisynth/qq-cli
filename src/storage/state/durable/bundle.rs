//! Publish a complete immutable file set with one no-replace directory rename.
use super::*;
use std::io::Read;

fn leaf(name: &str) -> Result<()> {
    if name.is_empty() || name.contains('/') || name == "." || name == ".." || name.contains('\0') {
        return Err(AppError::storage());
    }
    Ok(())
}

fn private(directory: &Directory) -> Result<()> {
    directory.verify()?;
    let meta = directory.file.metadata().map_err(|_| AppError::storage())?;
    if meta.uid() != rustix::process::geteuid().as_raw() || meta.mode() & 0o077 != 0 {
        return Err(AppError::storage());
    }
    Ok(())
}

pub(crate) fn publish_files_new(
    parent: &Path,
    name: &str,
    files: &[(&str, &[u8])],
    guard: impl Fn() -> Result<()>,
) -> Result<PathBuf> {
    leaf(name)?;
    let mut names = std::collections::BTreeSet::new();
    if files.is_empty() {
        return Err(AppError::storage());
    }
    for (name, _) in files {
        leaf(name)?;
        if !names.insert(*name) {
            return Err(AppError::storage());
        }
    }
    let parent = Directory::open(parent)?;
    private(&parent)?;
    guard()?;
    private(&parent)?;
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|_| AppError::storage())?;
    let staging = format!(".qq-cli-bundle-{:032x}.tmp", u128::from_le_bytes(random));
    rustix::fs::mkdirat(&parent.file, staging.as_str(), Mode::RWXU)
        .map_err(|_| AppError::storage())?;
    // Retain failed staging rather than recursively deleting a path which could
    // have been replaced. It is never a valid final bundle or an executable input.
    let directory = Directory {
        path: parent.path.join(&staging),
        file: openat(
            &parent.file,
            staging.as_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(File::from)
        .map_err(|_| AppError::storage())?,
    };
    private(&directory)?;
    let mut handles = Vec::new();
    for (name, data) in files {
        let mut file = directory.create_new(name)?;
        file.write_all(data).map_err(|_| AppError::storage())?;
        sync_file(&file)?;
        handles.push(file);
    }
    directory.file.sync_all().map_err(|_| AppError::storage())?;
    private(&directory)?;
    private(&parent)?;
    guard()?;
    private(&directory)?;
    private(&parent)?;
    rustix::fs::renameat_with(
        &parent.file,
        staging.as_str(),
        &parent.file,
        name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|_| AppError::storage())?;
    parent.file.sync_all().map_err(|_| AppError::storage())?;
    for file in &handles {
        sync_file(file)?;
    }
    private(&parent)?;
    guard()?;
    let published = parent.path.join(name);
    let current = Directory::open(&published)?
        .file
        .metadata()
        .map_err(|_| AppError::storage())?;
    let original = directory.file.metadata().map_err(|_| AppError::storage())?;
    if current.dev() != original.dev() || current.ino() != original.ino() {
        return Err(AppError::storage());
    }
    verify_files(&published, files)?;
    Ok(published)
}

pub(crate) fn verify_files(path: &Path, files: &[(&str, &[u8])]) -> Result<()> {
    let directory = Directory::open(path)?;
    private(&directory)?;
    let actual = std::fs::read_dir(path)
        .map_err(|_| AppError::storage())?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name())
                .map_err(|_| AppError::storage())
        })
        .collect::<Result<std::collections::BTreeSet<_>>>()?;
    let expected = files
        .iter()
        .map(|(name, _)| OsString::from(name))
        .collect::<std::collections::BTreeSet<_>>();
    if actual != expected {
        return Err(AppError::storage());
    }
    for (name, expected) in files {
        leaf(name)?;
        let mut file = openat(
            &directory.file,
            *name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map(File::from)
        .map_err(|_| AppError::storage())?;
        permissions::private_file(&file)?;
        let meta = file.metadata().map_err(|_| AppError::storage())?;
        if !meta.is_file()
            || meta.nlink() != 1
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.mode() & 0o077 != 0
            || meta.len() != expected.len() as u64
        {
            return Err(AppError::storage());
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(expected.len() as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::storage())?;
        let current = statat(&directory.file, *name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|_| AppError::storage())?;
        if bytes != *expected
            || current.st_dev as u64 != meta.dev()
            || current.st_ino != meta.ino()
            || current.st_nlink != 1
            || current.st_mode & 0o077 != 0
            || current.st_uid != meta.uid()
        {
            return Err(AppError::storage());
        }
        permissions::private_file(&file)?;
    }
    private(&directory)
}

//! Bounded directory snapshots preserve relative paths; their quota marker is not ownership.
use super::*;
use std::io::Write;
pub(crate) const MARKER: &str = ".qq-cli-directory";
const TAG: &[u8] = b"qq-cli-directory-v1\n";
const MAX_ENTRIES: usize = 256;
const MAX_DEPTH: usize = 16;
#[derive(Serialize, PartialEq, Eq)]
struct Member {
    path: String,
    directory: bool,
    size: u64,
    sha256: String,
}
fn failure() -> AppError {
    AppError::new(
        "DIRECTORY_INPUT_INVALID",
        "Directory input must be link-free and contain nonempty content within 16 levels, 256 entries and 128 MiB.",
        2,
    )
}
fn hash_file(path: &Path, deadline: Instant) -> Result<(u64, String)> {
    reject_links(path).map_err(|_| failure())?;
    let mut file = std::fs::File::open(path).map_err(|_| failure())?;
    let before = file.metadata().map_err(|_| failure())?;
    if !before.is_file() || before.len() > MAX_DOWNLOAD_BYTES {
        return Err(failure());
    }
    let mut hash = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 65536];
    loop {
        crate::core::deadline::check(deadline)?;
        let count = file.read(&mut buffer).map_err(|_| failure())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > before.len() {
            return Err(failure());
        }
        hash.update(&buffer[..count]);
    }
    let after = file.metadata().map_err(|_| failure())?;
    reject_links(path).map_err(|_| failure())?;
    if total != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(failure());
    }
    Ok((total, format!("{:x}", hash.finalize())))
}
fn walk(
    root: &Path,
    relative: &Path,
    depth: usize,
    members: &mut Vec<Member>,
    total: &mut u64,
    deadline: Instant,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    if depth > MAX_DEPTH {
        return Err(failure());
    }
    let current = root.join(relative);
    reject_links(&current).map_err(|_| failure())?;
    for entry in std::fs::read_dir(&current).map_err(|_| failure())? {
        crate::core::deadline::check(deadline)?;
        if depth >= MAX_DEPTH {
            return Err(failure());
        }
        let entry = entry.map_err(|_| failure())?;
        if members.len() >= MAX_ENTRIES {
            return Err(failure());
        }
        let name = entry.file_name().into_string().map_err(|_| failure())?;
        crate::napcat::install::archive::member_path(&name).map_err(|_| failure())?;
        let path = relative.join(name);
        let text = path.to_str().ok_or_else(failure)?.replace('\\', "/");
        if text.len() > 1024 {
            return Err(failure());
        }
        reject_links(&entry.path()).map_err(|_| failure())?;
        let metadata = std::fs::symlink_metadata(entry.path()).map_err(|_| failure())?;
        if metadata.is_dir() {
            members.push(Member {
                path: text,
                directory: true,
                size: 0,
                sha256: String::new(),
            });
            walk(root, &path, depth + 1, members, total, deadline)?;
        } else if metadata.is_file() {
            let (size, sha256) = hash_file(&entry.path(), deadline)?;
            *total = total.saturating_add(size);
            if *total > MAX_DOWNLOAD_BYTES {
                return Err(failure());
            }
            members.push(Member {
                path: text,
                directory: false,
                size,
                sha256,
            });
        } else {
            return Err(failure());
        }
    }
    reject_links(&current).map_err(|_| failure())?;
    Ok(())
}
fn inventory(root: &Path, deadline: Instant) -> Result<(Vec<Member>, Transfer)> {
    reject_links(root).map_err(|_| failure())?;
    if !root.is_dir() {
        return Err(failure());
    }
    let mut members = Vec::new();
    let mut size = 0;
    walk(root, Path::new(""), 0, &mut members, &mut size, deadline)?;
    members.sort_by(|a, b| a.path.cmp(&b.path));
    if size == 0 {
        return Err(failure());
    }
    let bytes = serde_json::to_vec(&members).map_err(|_| failure())?;
    Ok((
        members,
        Transfer {
            size_bytes: size,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        },
    ))
}
fn source(source: &Path) -> Result<(PathBuf, String)> {
    let path = std::path::absolute(source).map_err(|_| failure())?;
    reject_links(&path).map_err(|_| failure())?;
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(failure)?
        .to_owned();
    crate::napcat::install::archive::member_path(&name).map_err(|_| failure())?;
    if name.eq_ignore_ascii_case(MARKER) || name.eq_ignore_ascii_case(super::MARKER) {
        return Err(failure());
    }
    Ok((path, name))
}
pub(crate) fn validate(source_path: &Path, deadline: Instant) -> Result<()> {
    let (path, _) = source(source_path)?;
    inventory(&path, deadline).map(|_| ())
}
pub(crate) fn prepare(
    source_path: &Path,
    store: &Store,
    instance: &crate::storage::state::Instance,
    deadline: Instant,
) -> Result<Snapshot> {
    let (source, name) = source(source_path)?;
    let canonical_source = source.canonicalize().map_err(|_| failure())?;
    let mut state = std::path::absolute(store.root()).map_err(|_| failure())?;
    let mut tail = Vec::new();
    while !state.try_exists().map_err(|_| failure())? {
        crate::core::deadline::check(deadline)?;
        tail.push(state.file_name().ok_or_else(failure)?.to_owned());
        state = state.parent().ok_or_else(failure)?.to_owned();
    }
    reject_links(&state).map_err(|_| failure())?;
    state = state.canonicalize().map_err(|_| failure())?;
    for part in tail.into_iter().rev() {
        state.push(part);
    }
    if state.starts_with(&canonical_source) || canonical_source.starts_with(&state) {
        return Err(AppError::new(
            "DIRECTORY_STATE_OVERLAP",
            "Directory uploads cannot include the tool state or its private cache.",
            2,
        ));
    }
    let (members, transfer) = inventory(&source, deadline)?;
    let prefix = run_prefix(instance)?;
    let _transaction = store.transaction(deadline)?;
    cache::reserve(
        store,
        transfer.size_bytes + TAG.len() as u64,
        cache::MAX_CACHE_BYTES,
        deadline,
    )?;
    let staging = store.root().join("attachments");
    private_dir(&staging)?;
    let directory = tempfile::Builder::new()
        .prefix(&prefix)
        .tempdir_in(&staging)
        .map_err(|_| AppError::storage())?;
    crate::storage::state::restrict_private(directory.path())?;
    let path = directory.path().join(&name);
    private_dir(&path)?;
    let tag = directory.path().join(MARKER);
    let mut marker = std::fs::File::create_new(&tag).map_err(|_| AppError::storage())?;
    marker
        .write_all(TAG)
        .and_then(|_| marker.sync_all())
        .map_err(|_| AppError::storage())?;
    crate::storage::state::restrict_private(&tag)?;
    for member in &members {
        crate::core::deadline::check(deadline)?;
        let target = path.join(&member.path);
        if member.directory {
            private_dir(&target)?;
        } else {
            let copied = copy_from_cache(
                &source.join(&member.path),
                std::slice::from_ref(&source),
                &target,
                Some(member.size),
                deadline,
            )?;
            if copied.sha256 != member.sha256 {
                return Err(failure());
            }
        }
    }
    let (copied, _) = inventory(&path, deadline)?;
    let (after, _) = inventory(&source, deadline)?;
    if copied != members || after != members {
        return Err(failure());
    }
    Ok(Snapshot {
        directory,
        path,
        name,
        kind: Kind::File,
        transfer,
        source_directory: true,
    })
}
pub(super) fn verify(path: &Path, owned: &Owned, deadline: Instant) -> Result<()> {
    if !owned.persistent {
        return Err(ownership());
    }
    let (_, current) = inventory(path, deadline).map_err(|_| ownership())?;
    if current.size_bytes != owned.size || current.sha256 != owned.sha256 {
        return Err(ownership());
    }
    Ok(())
}
/// This enables bounded metadata accounting only. Signed a1 ownership is checked separately.
pub(crate) fn tagged(directory: &Path) -> Result<bool> {
    let path = directory.join(MARKER);
    reject_links(&path)?;
    let mut file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => return Err(AppError::storage()),
    };
    if !file.metadata().map_err(|_| AppError::storage())?.is_file() {
        return Ok(false);
    }
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(64)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::storage())?;
    Ok(bytes == TAG)
}
pub(crate) fn usage(path: &Path, deadline: Instant) -> Result<u64> {
    fn visit(
        path: &Path,
        depth: usize,
        count: &mut usize,
        bytes: &mut u64,
        deadline: Instant,
    ) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if depth > MAX_DEPTH || *count > MAX_ENTRIES {
            return Err(cache::quota_error());
        }
        reject_links(path)?;
        let metadata = std::fs::symlink_metadata(path).map_err(|_| AppError::storage())?;
        if metadata.is_file() {
            *bytes = bytes.saturating_add(metadata.len());
        } else if metadata.is_dir() {
            for entry in std::fs::read_dir(path).map_err(|_| AppError::storage())? {
                *count += 1;
                visit(
                    &entry.map_err(|_| AppError::storage())?.path(),
                    depth + 1,
                    count,
                    bytes,
                    deadline,
                )?;
            }
        } else {
            return Err(cache::quota_error());
        }
        Ok(())
    }
    let mut count = 0;
    let mut bytes = 0;
    visit(path, 0, &mut count, &mut bytes, deadline)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;

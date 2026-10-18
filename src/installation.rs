//! Installation ownership, crash-recoverable registration, and journaled removal.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::archive::{self, Member};
use crate::catalog::Release;
use crate::config::read_json;
use crate::domain::request_key;
use crate::error::{AppError, Result};
use crate::output::Envelope;
use crate::references::References;
use crate::state::{Installation, Store, atomic_json, private_dir, reject_links};

const MARKER: &str = ".qq-cli-installation.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Owned {
    installation_id: String,
    version: String,
    archive_digest: String,
    source_commit: String,
    source_url: String,
    authorization_evidence: String,
    directory: PathBuf,
    members: Vec<Member>,
    directories: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Removal {
    installation_id: String,
    manifest_reference: String,
}

pub fn import(
    reader: impl Read,
    release: &Release,
    store: &Store,
    references: &References,
    deadline: Instant,
) -> Result<Envelope> {
    release.require_authorization()?;
    let id = installation_id(release)?;
    let mut transaction = store.transaction(deadline)?;
    let parent = store.root().join("installations");
    private_dir(&parent)?;
    let parent = parent.canonicalize().map_err(|_| AppError::storage())?;
    let directory = parent.join(&id);
    reject_links(&directory)?;
    if has_pending(store, &id)? {
        return Err(busy());
    }
    let cached = directory.try_exists().map_err(|_| AppError::storage())?;
    if cached {
        let (_, owned) = load_owned(&directory, references)?;
        if owned.installation_id != id
            || owned.version != release.version
            || owned.archive_digest != release.archive.sha256
            || owned.directory != directory
        {
            return Err(modified());
        }
        verify_tree(&directory, &owned, false, deadline)?;
    } else {
        if transaction
            .registry
            .installations
            .iter()
            .any(|item| item.id == id)
        {
            return Err(modified());
        }
        let prepared = archive::prepare(reader, &release.archive, &parent, deadline)?;
        let (_, directories) = inventory(prepared.directory.path(), deadline)?;
        let owned = Owned {
            installation_id: id.clone(),
            version: release.version.into(),
            archive_digest: release.archive.sha256.into(),
            source_commit: release.commit.into(),
            source_url: release.url.into(),
            authorization_evidence: release
                .authorization_evidence
                .ok_or_else(AppError::invalid)?
                .into(),
            directory: directory.clone(),
            members: prepared.members,
            directories: directories.into_iter().collect(),
        };
        let reference = references.issue("i1", &owned)?;
        atomic_json(
            &prepared.directory.path().join(MARKER),
            &Pointer {
                schema_version: 1,
                reference,
            },
        )?;
        std::fs::rename(prepared.directory.path(), &directory).map_err(|_| AppError::storage())?;
        let _ = prepared.directory.keep();
    }
    if let Some(existing) = transaction
        .registry
        .installations
        .iter()
        .find(|item| item.id == id)
    {
        if existing.version != release.version
            || existing.digest != release.archive.sha256
            || existing.directory != directory
        {
            return Err(modified());
        }
    } else {
        transaction.registry.installations.push(Installation {
            id: id.clone(),
            version: release.version.into(),
            digest: release.archive.sha256.into(),
            directory,
        });
    }
    transaction.commit()?;
    Ok(Envelope::success(
        json!({"installationId":id,"version":release.version,"cached":cached}),
    ))
}

pub fn uninstall(
    store: &Store,
    id: &str,
    references: &References,
    deadline: Instant,
) -> Result<Envelope> {
    request_key(id).map_err(|_| AppError::invalid())?;
    let mut transaction = store.transaction(deadline)?;
    let users = transaction
        .registry
        .instances
        .iter()
        .filter(|instance| instance.installation_id == id)
        .map(|instance| instance.id.clone())
        .collect::<Vec<_>>();
    if !users.is_empty() {
        return Err(AppError::new(
            "INSTALLATION_IN_USE",
            "Installation is referenced by registered instances, including stopped ones.",
            4,
        )
        .with_details(
            json!({"instances":users.iter().take(20).collect::<Vec<_>>(),"count":users.len()}),
        ));
    }
    let journal = removal_path(store, id)?;
    let registered = transaction
        .registry
        .installations
        .iter()
        .find(|item| item.id == id);
    if registered.is_none() && !journal.try_exists().map_err(|_| AppError::storage())? {
        return Err(AppError::new(
            "INSTALLATION_NOT_FOUND",
            "Installation is not registered and has no removal journal.",
            4,
        ));
    }
    let parent = store.root().join("installations");
    reject_links(&parent)?;
    let parent = parent.canonicalize().map_err(|_| AppError::storage())?;
    let directory = parent.join(id);
    let trash = parent.join(format!(".removing-{id}"));
    reject_links(&directory)?;
    reject_links(&trash)?;
    if registered.is_some_and(|item| item.directory != directory) {
        return Err(modified());
    }
    let (manifest_reference, owned) = if journal.try_exists().map_err(|_| AppError::storage())? {
        let pointer = read_pointer(&journal)?;
        let removal: Removal = references.get("u1", &pointer.reference)?;
        if removal.installation_id != id {
            return Err(modified());
        }
        let owned: Owned = references.get("i1", &removal.manifest_reference)?;
        (removal.manifest_reference, owned)
    } else {
        let registered = registered.ok_or_else(|| {
            AppError::new(
                "INSTALLATION_NOT_FOUND",
                "Installation is not registered and has no removal journal.",
                4,
            )
        })?;
        let (manifest_reference, owned) = load_owned(&directory, references)?;
        if owned.version != registered.version || owned.archive_digest != registered.digest {
            return Err(modified());
        }
        verify_tree(&directory, &owned, false, deadline)?;
        private_dir(journal.parent().ok_or_else(AppError::storage)?)?;
        let reference = references.issue(
            "u1",
            &Removal {
                installation_id: id.into(),
                manifest_reference: manifest_reference.clone(),
            },
        )?;
        atomic_json(
            &journal,
            &Pointer {
                schema_version: 1,
                reference,
            },
        )?;
        (manifest_reference, owned)
    };
    if owned.installation_id != id || owned.directory != directory {
        return Err(modified());
    }
    if registered
        .is_some_and(|item| item.version != owned.version || item.digest != owned.archive_digest)
    {
        return Err(modified());
    }
    if directory.try_exists().map_err(|_| AppError::storage())? {
        if trash.try_exists().map_err(|_| AppError::storage())? {
            return Err(modified());
        }
        verify_tree(&directory, &owned, false, deadline)?;
        std::fs::rename(&directory, &trash).map_err(|_| AppError::storage())?;
    }
    if trash.try_exists().map_err(|_| AppError::storage())? {
        verify_tree(&trash, &owned, true, deadline)?;
        if trash.join(MARKER).exists()
            && read_pointer(&trash.join(MARKER))?.reference != manifest_reference
        {
            return Err(modified());
        }
        // Delete only authenticated members, never recursively empty an installation namespace.
        for member in &owned.members {
            check_deadline(deadline)?;
            let path = trash.join(archive::member_path(&member.path)?);
            reject_links(&path)?;
            if path.try_exists().map_err(|_| AppError::storage())? {
                std::fs::remove_file(path).map_err(|_| AppError::storage())?;
            }
        }
        let mut directories = owned.directories.clone();
        directories.sort_by_key(|path| std::cmp::Reverse(path.split('/').count()));
        for relative in directories {
            check_deadline(deadline)?;
            let path = trash.join(archive::member_path(&relative)?);
            reject_links(&path)?;
            if path.try_exists().map_err(|_| AppError::storage())? {
                std::fs::remove_dir(path).map_err(|_| AppError::storage())?;
            }
        }
        let marker = trash.join(MARKER);
        reject_links(&marker)?;
        if marker.try_exists().map_err(|_| AppError::storage())? {
            std::fs::remove_file(marker).map_err(|_| AppError::storage())?;
        }
        std::fs::remove_dir(&trash).map_err(|_| AppError::storage())?;
    }
    transaction
        .registry
        .installations
        .retain(|item| item.id != id);
    transaction.persist()?;
    std::fs::remove_file(journal).map_err(|_| AppError::storage())?;
    Ok(Envelope::success(
        json!({"installationId":id,"removed":true}),
    ))
}

pub fn verify(
    store: &Store,
    installation: &Installation,
    references: &References,
    deadline: Instant,
) -> Result<()> {
    request_key(&installation.id).map_err(|_| modified())?;
    let root = store.root().join("installations");
    reject_links(&root)?;
    let expected = root
        .canonicalize()
        .map_err(|_| AppError::storage())?
        .join(&installation.id);
    if installation.directory != expected {
        return Err(modified());
    }
    if has_pending(store, &installation.id)? {
        return Err(busy());
    }
    let (_, owned) = load_owned(&expected, references)?;
    if owned.installation_id != installation.id
        || owned.version != installation.version
        || owned.archive_digest != installation.digest
        || owned.directory != expected
    {
        return Err(modified());
    }
    verify_tree(&expected, &owned, false, deadline)
}

fn installation_id(release: &Release) -> Result<String> {
    if release.archive.sha256.len() != 64
        || !release
            .archive
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AppError::invalid());
    }
    let id = format!(
        "nc-{}-{}",
        release.version.trim_start_matches('v').replace('.', "-"),
        &release.archive.sha256[..16]
    );
    request_key(&id).map_err(|_| AppError::invalid())?;
    Ok(id)
}

fn load_owned(directory: &Path, references: &References) -> Result<(String, Owned)> {
    reject_links(directory)?;
    let pointer = read_pointer(&directory.join(MARKER))?;
    let owned = references.get("i1", &pointer.reference)?;
    Ok((pointer.reference, owned))
}
fn read_pointer(path: &Path) -> Result<Pointer> {
    reject_links(path)?;
    let pointer: Pointer = read_json(path, 1024)?;
    if pointer.schema_version != 1 {
        return Err(modified());
    }
    Ok(pointer)
}
fn removal_path(store: &Store, id: &str) -> Result<PathBuf> {
    request_key(id).map_err(|_| AppError::invalid())?;
    let path = store
        .root()
        .join("installation-operations")
        .join(format!("{id}.json"));
    reject_links(&path)?;
    Ok(path)
}

pub fn has_pending(store: &Store, id: &str) -> Result<bool> {
    removal_path(store, id)?
        .try_exists()
        .map_err(|_| AppError::storage())
}

fn inventory(root: &Path, deadline: Instant) -> Result<(BTreeSet<String>, BTreeSet<String>)> {
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut pending = vec![(root.to_owned(), 0_usize)];
    while let Some((path, depth)) = pending.pop() {
        if depth > 32 {
            return Err(modified());
        }
        check_deadline(deadline)?;
        reject_links(&path)?;
        for entry in std::fs::read_dir(path).map_err(|_| AppError::storage())? {
            let entry = entry.map_err(|_| AppError::storage())?;
            let path = entry.path();
            reject_links(&path)?;
            let relative = path
                .strip_prefix(root)
                .map_err(|_| modified())?
                .to_str()
                .ok_or_else(modified)?
                .replace('\\', "/");
            if relative == MARKER {
                continue;
            }
            archive::member_path(&relative)?;
            if entry.file_type().map_err(|_| AppError::storage())?.is_dir() {
                directories.insert(relative);
                pending.push((path, depth + 1));
            } else if entry
                .file_type()
                .map_err(|_| AppError::storage())?
                .is_file()
            {
                files.insert(relative);
            } else {
                return Err(modified());
            }
            if files.len() + directories.len() > 4096 {
                return Err(modified());
            }
        }
    }
    Ok((files, directories))
}

fn verify_tree(root: &Path, owned: &Owned, remaining: bool, deadline: Instant) -> Result<()> {
    let (files, directories) = inventory(root, deadline)?;
    let members = owned
        .members
        .iter()
        .map(|member| (member.path.as_str(), member))
        .collect::<BTreeMap<_, _>>();
    if members.len() != owned.members.len()
        || owned.members.len() > 4096
        || owned.directories.len() > 4096
    {
        return Err(modified());
    }
    let expected_files = members
        .keys()
        .map(|path| (*path).to_owned())
        .collect::<BTreeSet<_>>();
    let expected_directories = owned.directories.iter().cloned().collect::<BTreeSet<_>>();
    if !files.is_subset(&expected_files)
        || !directories.is_subset(&expected_directories)
        || !remaining && (files != expected_files || directories != expected_directories)
    {
        return Err(modified());
    }
    for relative in files {
        let member = members.get(relative.as_str()).ok_or_else(modified)?;
        let path = root.join(archive::member_path(&relative)?);
        reject_links(&path)?;
        let mut file = File::open(path).map_err(|_| AppError::storage())?;
        if file.metadata().map_err(|_| AppError::storage())?.len() != member.bytes
            || member.bytes > 128 * 1024 * 1024
        {
            return Err(modified());
        }
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; 65536];
        let mut size = 0_u64;
        loop {
            check_deadline(deadline)?;
            let count = file.read(&mut buffer).map_err(|_| AppError::storage())?;
            if count == 0 {
                break;
            }
            size += count as u64;
            if size > member.bytes {
                return Err(modified());
            }
            hash.update(&buffer[..count]);
        }
        if size != member.bytes || format!("{:x}", hash.finalize()) != member.sha256 {
            return Err(modified());
        }
    }
    Ok(())
}

fn check_deadline(deadline: Instant) -> Result<()> {
    if Instant::now() >= deadline {
        Err(AppError::new(
            "DEADLINE",
            "Installation operation exceeded its deadline; durable recovery records were retained.",
            3,
        ))
    } else {
        Ok(())
    }
}
fn modified() -> AppError {
    AppError::new(
        "INSTALLATION_MODIFIED",
        "Installation ownership or contents do not match; unknown files and data were preserved.",
        4,
    )
}
fn busy() -> AppError {
    AppError::new(
        "INSTALLATION_BUSY",
        "Installation has an unfinished removal operation; resume that operation before use.",
        4,
    )
}

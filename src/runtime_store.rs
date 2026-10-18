//! First-launch material preparation, never process control or adoption of running instances.
use crate::credentials::Credentials;
use crate::error::{AppError, Result};
use crate::references::References;
use crate::runtime_config::{Bundle, Ports};
use crate::state::{Registry, Store, Transaction, private_dir, reject_links, restrict_private};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
const MARKER: &str = ".qq-cli-runtime.json";
const DIRS: [&str; 5] = ["config", "tmp", "logs", "cache", "plugins"];

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Plan {
    pub instance_id: String,
    pub run_id: String,
    pub version: String,
    pub ports: Ports,
}
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Owned {
    schema_version: u8,
    template_version: u8,
    plan: Plan,
    directory: PathBuf,
    installation_id: String,
    installation_digest: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Pointer {
    schema_version: u8,
    reference: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Secrets {
    schema_version: u8,
    reference: String,
    api: String,
    control: String,
    jwt: String,
}
/// Only returned after exact material verification and a committed prepared state.
/// This is not a launch permit; lifecycle must first persist its starting intent.
pub struct Prepared {
    directory: PathBuf,
    bundle: Bundle,
    api_key: String,
}
impl Prepared {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }
    pub fn api_credential_key(&self) -> &str {
        &self.api_key
    }
}
pub fn api_credential_key(instance_id: &str, run_id: &str) -> Result<String> {
    Ok(format!("runtime-api-{}", identity(instance_id, run_id)?))
}
fn identity(instance_id: &str, run_id: &str) -> Result<String> {
    crate::domain::request_key(instance_id).map_err(|_| AppError::invalid())?;
    crate::domain::request_key(run_id).map_err(|_| AppError::invalid())?;
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&(instance_id, run_id)).map_err(|_| AppError::protocol())?
        )
    ))
}
fn eligible(registry: &Registry, plan: &Plan) -> Result<(String, String)> {
    eligible_in(registry, plan, &["preparing", "prepared"])
}
fn eligible_in(registry: &Registry, plan: &Plan, phases: &[&str]) -> Result<(String, String)> {
    let instance = registry
        .instances
        .iter()
        .find(|i| i.id == plan.instance_id)
        .ok_or_else(conflict)?;
    if instance.run_id != plan.run_id
        || !phases.contains(&instance.state.as_str())
        || instance.account.is_some()
        || instance.process_id.is_some()
        || instance.process_started.is_some()
        || instance.endpoint != format!("http://127.0.0.1:{}/", plan.ports.http)
        || instance.ws_endpoint.as_deref()
            != Some(format!("ws://127.0.0.1:{}/event", plan.ports.events).as_str())
        || instance.credential_key != api_credential_key(&plan.instance_id, &plan.run_id)?
    {
        return Err(conflict());
    }
    let install = registry
        .installations
        .iter()
        .find(|i| {
            i.id == instance.installation_id
                && i.version.strip_prefix('v').unwrap_or(&i.version)
                    == plan.version.strip_prefix('v').unwrap_or(&plan.version)
        })
        .ok_or_else(conflict)?;
    Ok((install.id.clone(), install.digest.clone()))
}

/// Read-only final material check in the caller's lifecycle transaction. Missing
/// secrets/files are never regenerated, repaired, or treated as a new preparation.
/// This must run before dispatch; live mutable runtime directories are unsupported.
pub fn verify_prepared(
    transaction: &Transaction<'_>,
    refs: &References,
    credentials: &impl Credentials,
    plan: &Plan,
    deadline: Instant,
) -> Result<Prepared> {
    crate::deadline::check(deadline)?;
    let mut plan = plan.clone();
    plan.version = crate::catalog::lookup(&plan.version)?.version.into();
    plan.ports.validate()?;
    let id = identity(&plan.instance_id, &plan.run_id)?;
    let (installation_id, installation_digest) =
        eligible_in(&transaction.registry, &plan, &["prepared", "starting"])?;
    let base = transaction
        .store()
        .root()
        .canonicalize()
        .map_err(|_| conflict())?;
    let directory = base.join("runtimes").join(&id);
    let stage = base.join("runtimes").join(format!("{id}.stage"));
    let journal = base.join("runtime-preparations").join(format!("{id}.json"));
    for path in [&directory, &stage, &journal] {
        reject_links(path)?;
    }
    if stage.try_exists().map_err(|_| conflict())? {
        return Err(conflict());
    }
    let pointer: Pointer = crate::config::read_json(&journal, 4096)?;
    let expected = Owned {
        schema_version: 1,
        template_version: 1,
        plan: plan.clone(),
        directory: directory.clone(),
        installation_id,
        installation_digest,
    };
    if pointer.schema_version != 1 || refs.get::<Owned>("r1", &pointer.reference)? != expected {
        return Err(conflict());
    }
    let raw = optional(credentials, &format!("runtime-bundle-{id}"), deadline)?
        .ok_or_else(credential_conflict)?;
    if raw.len() > 2048 {
        return Err(credential_conflict());
    }
    let secrets: Secrets = serde_json::from_str(&raw).map_err(|_| credential_conflict())?;
    if secrets.schema_version != 1 || secrets.reference != pointer.reference {
        return Err(credential_conflict());
    }
    let bundle = Bundle::restore(
        &plan.version,
        plan.ports,
        secrets.api,
        secrets.control,
        secrets.jwt,
    )?;
    let api_key = api_credential_key(&plan.instance_id, &plan.run_id)?;
    if optional(credentials, &api_key, deadline)?.as_deref() != Some(bundle.api_token()) {
        return Err(credential_conflict());
    }
    let mut files = BTreeMap::new();
    for file in bundle.files() {
        let mut bytes = Vec::new();
        file.write_to(&mut bytes)?;
        files.insert(file.name().to_owned(), bytes);
    }
    let marker = serde_json::to_vec(&pointer).map_err(|_| conflict())?;
    inspect(&directory, &files, &marker, true, deadline)?;
    crate::deadline::check(deadline)?;
    Ok(Prepared {
        directory,
        bundle,
        api_key,
    })
}
pub fn prepare(
    store: &Store,
    refs: &References,
    credentials: &impl Credentials,
    plan: &Plan,
    deadline: Instant,
) -> Result<Prepared> {
    crate::deadline::check(deadline)?;
    if !cfg!(windows) {
        return Err(AppError::new(
            "PLATFORM_UNSUPPORTED",
            "Runtime directory publication is supported on Windows only.",
            4,
        ));
    }
    let mut plan = plan.clone();
    plan.version = crate::catalog::lookup(&plan.version)?.version.into();
    plan.ports.validate()?;
    let id = identity(&plan.instance_id, &plan.run_id)?;
    eligible(&store.read()?, &plan)?;
    let mut transaction = store.transaction(deadline)?;
    crate::deadline::check(deadline)?;
    let (installation_id, installation_digest) = eligible(&transaction.registry, &plan)?;
    let base = store
        .root()
        .canonicalize()
        .map_err(|_| AppError::storage())?;
    let runtimes = base.join("runtimes");
    let local = base.to_str().ok_or_else(conflict)?;
    let local = local.strip_prefix("\\\\?\\").unwrap_or(local).as_bytes();
    if local.len() < 3 || !local[0].is_ascii_alphabetic() || local[1] != b':' || local[2] != b'\\' {
        return Err(conflict());
    }
    let journals = base.join("runtime-preparations");
    crate::deadline::check(deadline)?;
    private_dir(&runtimes)?;
    private_dir(&journals)?;
    let directory = runtimes.join(&id);
    let stage = runtimes.join(format!("{id}.stage"));
    let journal = journals.join(format!("{id}.json"));
    for path in [&directory, &stage, &journal] {
        reject_links(path)?;
    }
    let expected = Owned {
        schema_version: 1,
        template_version: 1,
        plan: plan.clone(),
        directory: directory.clone(),
        installation_id,
        installation_digest,
    };
    let bundle_key = format!("runtime-bundle-{id}");
    let api_key = api_credential_key(&plan.instance_id, &plan.run_id)?;
    let pointer = if journal.try_exists().map_err(|_| AppError::storage())? {
        let pointer: Pointer = crate::config::read_json(&journal, 4096)?;
        if pointer.schema_version != 1 || refs.get::<Owned>("r1", &pointer.reference)? != expected {
            return Err(conflict());
        }
        pointer
    } else {
        if directory.exists() || stage.exists() {
            return Err(conflict());
        }
        if optional(credentials, &bundle_key, deadline)?.is_some()
            || optional(credentials, &api_key, deadline)?.is_some()
        {
            return Err(credential_conflict());
        }
        let pointer = Pointer {
            schema_version: 1,
            reference: refs.issue("r1", &expected)?,
        };
        publish_pointer(&journal, &pointer, deadline)?;
        pointer
    };
    crate::deadline::check(deadline)?;
    let bundle = match optional(credentials, &bundle_key, deadline)? {
        Some(raw) => {
            if raw.len() > 2048 {
                return Err(credential_conflict());
            }
            let secrets: Secrets = serde_json::from_str(&raw).map_err(|_| credential_conflict())?;
            if secrets.schema_version != 1 || secrets.reference != pointer.reference {
                return Err(credential_conflict());
            }
            Bundle::restore(
                &plan.version,
                plan.ports,
                secrets.api,
                secrets.control,
                secrets.jwt,
            )?
        }
        None => {
            if directory.exists()
                || stage.exists()
                || optional(credentials, &api_key, deadline)?.is_some()
            {
                return Err(credential_conflict());
            }
            let bundle = Bundle::generate(&plan.version, plan.ports)?;
            let secret = Secrets {
                schema_version: 1,
                reference: pointer.reference.clone(),
                api: bundle.api_token().into(),
                control: bundle.control_token().into(),
                jwt: bundle.jwt_secret().into(),
            };
            save(
                credentials,
                &bundle_key,
                &serde_json::to_string(&secret).map_err(|_| AppError::protocol())?,
                deadline,
            )?;
            bundle
        }
    };
    match optional(credentials, &api_key, deadline)? {
        Some(token) if token != bundle.api_token() => return Err(credential_conflict()),
        Some(_) => (),
        None => save(credentials, &api_key, bundle.api_token(), deadline)?,
    }
    crate::deadline::check(deadline)?;
    let mut files = BTreeMap::new();
    for file in bundle.files() {
        let mut bytes = Vec::new();
        file.write_to(&mut bytes)?;
        files.insert(file.name().to_owned(), bytes);
    }
    let marker = serde_json::to_vec(&pointer).map_err(|_| AppError::protocol())?;
    if directory.exists() {
        if stage.exists() {
            return Err(conflict());
        }
        inspect(&directory, &files, &marker, true, deadline)?;
    } else {
        inspect(&stage, &files, &marker, false, deadline)?;
        crate::deadline::check(deadline)?;
        private_dir(&stage)?;
        for child in DIRS {
            crate::deadline::check(deadline)?;
            private_dir(&stage.join(child))?;
        }
        for (name, bytes) in &files {
            write_claimed(&stage.join("config").join(name), bytes, deadline)?;
        }
        write_claimed(&stage.join(MARKER), &marker, deadline)?;
        inspect(&stage, &files, &marker, true, deadline)?;
        crate::deadline::check(deadline)?;
        // Explicit Win32 MoveFileW does not replace an existing destination.
        // Keep this guarantee independent of std::fs::rename platform changes.
        reject_links(&stage)?;
        reject_links(&directory)?;
        if directory.exists()
            || stage.parent() != Some(runtimes.as_path())
            || directory.parent() != Some(runtimes.as_path())
        {
            return Err(conflict());
        }
        publish_directory(&stage, &directory)?;
        inspect(&directory, &files, &marker, true, deadline)?;
    }
    crate::deadline::check(deadline)?;
    transaction
        .registry
        .instances
        .iter_mut()
        .find(|i| i.id == plan.instance_id)
        .ok_or_else(conflict)?
        .state = "prepared".into();
    transaction.commit()?;
    Ok(Prepared {
        directory,
        bundle,
        api_key,
    })
}
fn optional(
    credentials: &impl Credentials,
    key: &str,
    deadline: Instant,
) -> Result<Option<String>> {
    crate::deadline::check(deadline)?;
    let result = match credentials.get(key) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code == "CREDENTIAL_NOT_FOUND" => Ok(None),
        Err(error) => Err(error),
    };
    crate::deadline::check(deadline)?;
    result
}
fn save(credentials: &impl Credentials, key: &str, value: &str, deadline: Instant) -> Result<()> {
    crate::deadline::check(deadline)?;
    credentials.set(key, value)?;
    crate::deadline::check(deadline)
}
fn publish_pointer(path: &Path, pointer: &Pointer, deadline: Instant) -> Result<()> {
    crate::deadline::check(deadline)?;
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(conflict)?)
        .map_err(|_| AppError::storage())?;
    restrict_private(temporary.path())?;
    serde_json::to_writer(&mut temporary, pointer).map_err(|_| AppError::storage())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| AppError::storage())?;
    crate::deadline::check(deadline)?;
    temporary
        .persist_noclobber(path)
        .map_err(|_| AppError::storage())?;
    Ok(())
}
fn read(path: &Path) -> Result<Vec<u8>> {
    reject_links(path)?;
    if !path.is_file() {
        return Err(conflict());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| conflict())?
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| conflict())?;
    if bytes.len() > 16384 {
        return Err(conflict());
    }
    Ok(bytes)
}
fn inspect(
    path: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    marker: &[u8],
    complete: bool,
    deadline: Instant,
) -> Result<()> {
    reject_links(path)?;
    if !path.exists() {
        return if complete { Err(conflict()) } else { Ok(()) };
    }
    if !path.is_dir() {
        return Err(conflict());
    }
    let mut present = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(path).map_err(|_| conflict())?.take(7) {
        crate::deadline::check(deadline)?;
        let entry = entry.map_err(|_| conflict())?;
        let name = entry.file_name().into_string().map_err(|_| conflict())?;
        let child = entry.path();
        reject_links(&child)?;
        if name == MARKER {
            let bytes = read(&child)?;
            if (complete && bytes != marker) || !marker.starts_with(&bytes) {
                return Err(conflict());
            }
        } else if DIRS.contains(&name.as_str()) && child.is_dir() {
            if name == "config" {
                let mut names = std::collections::BTreeSet::new();
                for file in std::fs::read_dir(&child).map_err(|_| conflict())?.take(4) {
                    let file = file.map_err(|_| conflict())?;
                    let name = file.file_name().into_string().map_err(|_| conflict())?;
                    let expected = files.get(&name).ok_or_else(conflict)?;
                    let bytes = read(&file.path())?;
                    if (complete && &bytes != expected) || !expected.starts_with(&bytes) {
                        return Err(conflict());
                    }
                    names.insert(name);
                }
                if complete && names.len() != files.len() {
                    return Err(conflict());
                }
            } else if std::fs::read_dir(child)
                .map_err(|_| conflict())?
                .next()
                .is_some()
            {
                return Err(conflict());
            }
        } else {
            return Err(conflict());
        }
        present.insert(name);
    }
    if complete && (present.len() != 6 || !present.contains(MARKER)) {
        return Err(conflict());
    }
    Ok(())
}
/// Only claimed stage members may be repaired, and only by appending the exact
/// missing suffix. Unknown, modified or extra bytes are never overwritten.
fn write_claimed(path: &Path, expected: &[u8], deadline: Instant) -> Result<()> {
    crate::deadline::check(deadline)?;
    reject_links(path)?;
    let mut file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|_| conflict())?,
        Err(_) => return Err(conflict()),
    };
    restrict_private(path)?;
    let mut current = Vec::new();
    (&mut file)
        .take(16385)
        .read_to_end(&mut current)
        .map_err(|_| conflict())?;
    if !expected.starts_with(&current) {
        return Err(conflict());
    }
    crate::deadline::check(deadline)?;
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(&expected[current.len()..]))
        .and_then(|_| file.sync_all())
        .map_err(|_| AppError::storage())?;
    Ok(())
}
#[cfg(windows)]
fn publish_directory(source: &Path, target: &Path) -> Result<()> {
    winsafe::MoveFile(
        source.to_str().ok_or_else(conflict)?,
        target.to_str().ok_or_else(conflict)?,
    )
    .map_err(|_| AppError::storage())
}
#[cfg(not(windows))]
fn publish_directory(_source: &Path, _target: &Path) -> Result<()> {
    Err(AppError::new(
        "PLATFORM_UNSUPPORTED",
        "Runtime directory publication is supported on Windows only.",
        4,
    ))
}

#[cfg(all(test, windows))]
#[test]
fn native_directory_publication_never_replaces_an_existing_file_or_directory() {
    for directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let source = root.join("source");
        let target = root.join("target");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("owned"), b"original owned bytes").unwrap();
        if directory {
            std::fs::create_dir(&target).unwrap();
        } else {
            std::fs::write(&target, b"preserve target").unwrap();
        }
        assert_eq!(source.parent(), Some(root.as_path()));
        assert_eq!(target.parent(), Some(root.as_path()));
        assert!(publish_directory(&source, &target).is_err());
        assert_eq!(
            std::fs::read(source.join("owned")).unwrap(),
            b"original owned bytes"
        );
        if directory {
            assert!(target.is_dir());
        } else {
            assert_eq!(std::fs::read(target).unwrap(), b"preserve target");
        }
    }
}

fn conflict() -> AppError {
    AppError::new(
        "RUNTIME_PREPARATION_CONFLICT",
        "Runtime ownership, phase or prepared contents do not match; unknown material was preserved.",
        4,
    )
}
fn credential_conflict() -> AppError {
    AppError::new(
        "RUNTIME_CREDENTIAL_CONFLICT",
        "Runtime credentials are missing or do not match their original preparation; no secrets were replaced.",
        4,
    )
}

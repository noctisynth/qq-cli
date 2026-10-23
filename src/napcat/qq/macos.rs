//! Read-only selected-payload inspection; not a bundle signature or launch permit.
mod macho;
use super::{Application, entry, metadata_error, split_version};
use crate::core::error::Result;
use crate::storage::state::reject_links;
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{Read, Seek};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;

struct Payload {
    path: PathBuf,
    file: File,
    metadata: Metadata,
}

/// Static original-app observation. It does not permit modifying or starting it.
pub struct Original {
    application: Application,
    origin: crate::native::macos_signature::Origin,
}
impl Original {
    pub fn application(&self) -> &Application {
        &self.application
    }
    pub fn origin(&self) -> crate::native::macos_signature::Origin {
        self.origin
    }
}

pub fn inspect_original(executable: &Path, deadline: Instant) -> Result<Original> {
    let application = inspect(executable, deadline)?;
    let root = application
        .executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(metadata_error)?;
    let origin = crate::native::macos_signature::verify_isolated(root, deadline)?;
    let repeated = inspect(&application.executable, deadline)?;
    if repeated.fingerprint != application.fingerprint || repeated.id != application.id {
        return Err(metadata_error());
    }
    Ok(Original {
        application,
        origin,
    })
}
impl Payload {
    fn open(path: PathBuf, limit: u64) -> Result<Self> {
        reject_links(&path)?;
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(&path)
            .map_err(|_| metadata_error())?;
        let metadata = file.metadata().map_err(|_| metadata_error())?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > limit {
            return Err(metadata_error());
        }
        let payload = Self {
            path,
            file,
            metadata,
        };
        payload.verify()?;
        Ok(payload)
    }
    fn verify(&self) -> Result<()> {
        reject_links(&self.path)?;
        let now = std::fs::symlink_metadata(&self.path).map_err(|_| metadata_error())?;
        let a = &self.metadata;
        if !now.is_file()
            || now.nlink() != 1
            || (
                now.dev(),
                now.ino(),
                now.len(),
                now.mode(),
                now.uid(),
                now.gid(),
                now.mtime(),
                now.mtime_nsec(),
                now.ctime(),
                now.ctime_nsec(),
            ) != (
                a.dev(),
                a.ino(),
                a.len(),
                a.mode(),
                a.uid(),
                a.gid(),
                a.mtime(),
                a.mtime_nsec(),
                a.ctime(),
                a.ctime_nsec(),
            )
        {
            return Err(metadata_error());
        }
        Ok(())
    }
    fn hash(&mut self, deadline: Instant) -> Result<String> {
        self.file.rewind().map_err(|_| metadata_error())?;
        let mut hash = Sha256::new();
        let mut total = 0;
        let mut bytes = [0; 65536];
        loop {
            crate::core::deadline::check(deadline)?;
            let count = self.file.read(&mut bytes).map_err(|_| metadata_error())?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > self.metadata.len() {
                return Err(metadata_error());
            }
            hash.update(&bytes[..count]);
        }
        if total != self.metadata.len() {
            return Err(metadata_error());
        }
        self.verify()?;
        Ok(format!("{:x}", hash.finalize()))
    }
}

pub fn inspect(executable: &Path, deadline: Instant) -> Result<Application> {
    crate::core::deadline::check(deadline)?;
    reject_links(executable)?;
    let executable = executable.canonicalize().map_err(|_| metadata_error())?;
    let macos = executable.parent().ok_or_else(metadata_error)?;
    let contents = macos.parent().ok_or_else(metadata_error)?;
    let root = contents.parent().ok_or_else(metadata_error)?;
    if executable.file_name().is_none_or(|name| name != "QQ")
        || macos.file_name().is_none_or(|name| name != "MacOS")
        || contents.file_name().is_none_or(|name| name != "Contents")
        || root.extension().is_none_or(|ext| ext != "app")
    {
        return Err(metadata_error());
    }
    let resources = contents.join("Resources/app");
    let paths = [
        executable.clone(),
        resources.join("major.node"),
        resources.join("wrapper.node"),
        resources.join("application.asar"),
        resources.join("package.json"),
        contents.join("Info.plist"),
    ];
    let mut files = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            Payload::open(
                path,
                if index >= 4 {
                    1024 * 1024
                } else {
                    512 * 1024 * 1024
                },
            )
        })
        .collect::<Result<Vec<_>>>()?;
    if files.iter().map(|file| file.metadata.len()).sum::<u64>() > 1024 * 1024 * 1024 {
        return Err(metadata_error());
    }
    for (index, kind) in [(0, 2), (1, 8), (2, 6)] {
        let payload = &mut files[index];
        macho::arm64(&mut payload.file, payload.metadata.len(), kind)?;
    }
    let mut json = Vec::new();
    let limit = files[4].metadata.len();
    (&mut files[4].file)
        .take(limit + 1)
        .read_to_end(&mut json)
        .map_err(|_| metadata_error())?;
    if json.len() as u64 != limit {
        return Err(metadata_error());
    }
    let package: serde_json::Value = serde_json::from_slice(&json).map_err(|_| metadata_error())?;
    let field = |name| {
        package
            .get(name)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(metadata_error)
    };
    if field("name")? != "qq-chat"
        || field("platform")? != "darwin"
        || field("main")? != "./application.asar/app_launcher/index.js"
    {
        return Err(metadata_error());
    }
    let version = field("version")?;
    let (_, embedded) = split_version(version)?;
    let build = field("buildVersion")?;
    if build.len() > 10
        || !build.bytes().all(|b| b.is_ascii_digit())
        || build.parse::<u32>().ok().is_none_or(|v| v == 0)
        || embedded.is_some_and(|value| value != build)
    {
        return Err(metadata_error());
    }
    entry::resolve(&resources, field("main")?)?;
    let hashes = files
        .iter_mut()
        .map(|file| file.hash(deadline))
        .collect::<Result<Vec<_>>>()?;
    for file in &files {
        file.verify()?;
    }
    let metadata_sha256 = digest(&[
        "macos-selected-metadata-v1",
        &hashes[2],
        &hashes[4],
        &hashes[5],
    ]);
    let fingerprint = digest(&[
        "macos-selected-application-v1",
        version,
        build,
        "aarch64",
        &hashes[0],
        &hashes[1],
        &hashes[3],
        &metadata_sha256,
    ]);
    Ok(Application {
        id: format!(
            "qq-{}",
            &digest(&[
                "macos-app-path-v1",
                root.to_str().ok_or_else(metadata_error)?
            ])[..16]
        ),
        version: version.into(),
        build: build.into(),
        architecture: "aarch64".into(),
        fingerprint,
        executable_sha256: hashes[0].clone(),
        major_sha256: hashes[1].clone(),
        entry_sha256: hashes[3].clone(),
        metadata_sha256,
        executable,
    })
}

fn digest(fields: &[&str]) -> String {
    let mut hash = Sha256::new();
    for field in fields {
        hash.update((field.len() as u64).to_le_bytes());
        hash.update(field.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

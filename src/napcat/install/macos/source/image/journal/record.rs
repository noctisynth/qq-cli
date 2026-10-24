use super::*;
use crate::storage::{config::read_json, state::Transaction};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    path: PathBuf,
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    digest: String,
}
impl Input {
    pub(super) fn inspect(store: &Store, image: &Image) -> Result<Self> {
        let input = Self {
            path: image.path.clone(),
            device: image.stamp.device,
            inode: image.stamp.inode,
            size: image.stamp.size,
            modified: image.stamp.modified,
            digest: image.digest.clone(),
        };
        input.verify_path(store)?;
        acquire::private_file(&image.file)?;
        Ok(input)
    }
    pub(super) fn verify_path(&self, store: &Store) -> Result<()> {
        let cache = store.root().join("qq-images");
        acquire::directory(&cache)?;
        if self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || self.path != cache.join(format!("{}.dmg", self.digest))
        {
            return Err(conflict());
        }
        let file = open(&self.path)?;
        acquire::private_file(&file)?;
        let stamp = stamp(&file)?;
        if (stamp.device, stamp.inode, stamp.size, stamp.modified)
            != (self.device, self.inode, self.size, self.modified)
        {
            return Err(conflict());
        }
        Ok(())
    }
    fn same_image(&self, other: &Self) -> bool {
        self.path == other.path || (self.device, self.inode) == (other.device, other.inode)
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Pending,
    Unknown,
    Release {
        request: String,
        fingerprint: String,
        receipt: mounted::Receipt,
        complete: bool,
    },
    Cleanup {
        request: String,
        fingerprint: String,
        receipt: mounted::Receipt,
        phase: CleanupPhase,
    },
    Complete {
        fingerprint: String,
        receipt: mounted::Receipt,
    },
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CleanupPhase {
    Mount,
    Parent,
    Complete,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub version: u8,
    pub request: String,
    pub input: Input,
    pub before: String,
    pub parent: (u64, u64),
    #[serde(default)]
    pub mount: Option<(u64, u64)>,
    pub outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pointer {
    version: u8,
    reference: String,
}
pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn identity(file: &File) -> Result<(u64, u64)> {
    let m = file.metadata().map_err(|_| conflict())?;
    Ok((m.dev(), m.ino()))
}
pub(super) fn create_directory(path: &Path) -> Result<()> {
    let parent = acquire::directory(path.parent().ok_or_else(conflict)?)?;
    rustix::fs::mkdirat(&parent, path.file_name().ok_or_else(conflict)?, Mode::RWXU)
        .map_err(|_| conflict())?;
    acquire::directory(path)?
        .sync_all()
        .map_err(|_| conflict())?;
    parent.sync_all().map_err(|_| conflict())?;
    if identity(&parent)? != identity(&acquire::directory(path.parent().ok_or_else(conflict)?)?)? {
        return Err(conflict());
    }
    Ok(())
}
pub(super) fn load(path: &Path, references: &References) -> Result<Option<Record>> {
    reject_links(path)?;
    let pointer: Pointer = match read_json(path, 4096) {
        Ok(v) => v,
        Err(e) if e.code == "FILE_NOT_FOUND" => return Ok(None),
        Err(e) => return Err(e),
    };
    if pointer.version != 1 {
        return Err(conflict());
    }
    let record: Record = references.get("m2", &pointer.reference)?;
    crate::core::domain::request_key(&record.request).map_err(|_| conflict())?;
    if !matches!(record.version, 1 | 2)
        || (record.version == 2 && record.mount.is_none())
        || path.file_name().and_then(|p| p.to_str())
            != Some(format!("{}.json", hash(record.request.as_bytes())).as_str())
    {
        return Err(conflict());
    }
    Ok(Some(record))
}
pub(super) fn save(
    transaction: &Transaction<'_>,
    lock: &Lock,
    references: &References,
    path: &Path,
    record: &Record,
) -> Result<()> {
    transaction.verify_lock()?;
    lock.verify()?;
    let reference = references.issue("m2", record)?;
    transaction.verify_lock()?;
    lock.verify()?;
    atomic_json(
        path,
        &Pointer {
            version: 1,
            reference,
        },
    )?;
    transaction.verify_lock()?;
    lock.verify()
}
pub(super) fn exclusive(
    directory: &Path,
    references: &References,
    request: &str,
    input: &Input,
) -> Result<()> {
    for (index, entry) in std::fs::read_dir(directory)
        .map_err(|_| conflict())?
        .enumerate()
    {
        if index >= 4096 {
            return Err(conflict());
        }
        let path = entry.map_err(|_| conflict())?.path();
        let record = load(&path, references)?.ok_or_else(conflict)?;
        if record.request != request
            && record.input.same_image(input)
            && !matches!(
                record.outcome,
                Outcome::Cleanup {
                    phase: CleanupPhase::Complete,
                    ..
                }
            )
        {
            return Err(conflict());
        }
    }
    Ok(())
}

pub(super) fn parent(active: &Path, input: &Input, request: &str, version: u8) -> Result<PathBuf> {
    let bytes = match version {
        1 => serde_json::to_vec(input),
        2 => serde_json::to_vec(&(input, request)),
        _ => return Err(conflict()),
    }
    .map_err(|_| conflict())?;
    Ok(active.join(hash(&bytes)))
}

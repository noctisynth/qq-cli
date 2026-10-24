use super::*;
use rustix::fs::{Mode, OFlags};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    os::{fd::AsFd, macos::fs::MetadataExt as _, unix::fs::MetadataExt},
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    device: u64,
    inode: u64,
    uid: u32,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    parent: Node,
    nodes: Vec<Node>,
}

fn flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}
fn node(file: &File) -> Result<Node> {
    let meta = file.metadata().map_err(|_| invalid())?;
    if !meta.is_dir()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o7777 != 0o700
        || meta.st_flags() != 0
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(file.as_fd())?;
    Ok(Node {
        device: meta.dev(),
        inode: meta.ino(),
        uid: meta.uid(),
    })
}
fn open_parent(root: &Path) -> Result<File> {
    let parent = root.parent().ok_or_else(invalid)?;
    reject_links(parent)?;
    if !parent.is_absolute() || parent.canonicalize().map_err(|_| invalid())? != parent {
        return Err(invalid());
    }
    rustix::fs::open(parent, flags(), Mode::empty())
        .map(File::from)
        .map_err(|_| invalid())
}
fn paths() -> Vec<PathBuf> {
    std::iter::once(PathBuf::new())
        .chain(super::super::macos_adapter::layout_directories())
        .collect()
}
impl Snapshot {
    pub(super) fn verify(&self, root: &Path, empty: bool, deadline: Instant) -> Result<()> {
        let paths = paths();
        if self.nodes.len() != paths.len() {
            return Err(invalid());
        }
        let parent = open_parent(root)?;
        if node(&parent)? != self.parent {
            return Err(invalid());
        }
        let mut held: BTreeMap<PathBuf, File> = BTreeMap::new();
        for (relative, expected) in paths.iter().zip(&self.nodes) {
            crate::core::deadline::check(deadline)?;
            let (directory, name) = if relative.as_os_str().is_empty() {
                (&parent, root.file_name().ok_or_else(invalid)?)
            } else {
                (
                    held.get(relative.parent().ok_or_else(invalid)?)
                        .ok_or_else(invalid)?,
                    relative.file_name().ok_or_else(invalid)?,
                )
            };
            let file = File::from(
                rustix::fs::openat(directory, name, flags(), Mode::empty())
                    .map_err(|_| invalid())?,
            );
            if node(&file)? != *expected || expected.device != self.parent.device {
                return Err(invalid());
            }
            held.insert(relative.clone(), file);
        }
        for (relative, file) in &held {
            crate::core::deadline::check(deadline)?;
            if empty {
                let expected: BTreeSet<_> = paths
                    .iter()
                    .filter(|p| !p.as_os_str().is_empty() && p.parent() == Some(relative.as_path()))
                    .map(|p| p.file_name().unwrap().to_owned())
                    .collect();
                let mut actual = BTreeSet::new();
                for entry in rustix::fs::Dir::read_from(file).map_err(|_| invalid())? {
                    let entry = entry.map_err(|_| invalid())?;
                    let name = entry.file_name().to_bytes();
                    if name == b"." || name == b".." {
                        continue;
                    }
                    use std::os::unix::ffi::OsStrExt;
                    actual.insert(std::ffi::OsStr::from_bytes(name).to_owned());
                    if actual.len() > paths.len() {
                        return Err(invalid());
                    }
                }
                if actual != expected {
                    return Err(invalid());
                }
            }
            // Reopen via the same original chain to detect exchanged paths.
            let current = if relative.as_os_str().is_empty() {
                rustix::fs::openat(
                    &parent,
                    root.file_name().ok_or_else(invalid)?,
                    flags(),
                    Mode::empty(),
                )
            } else {
                rustix::fs::openat(
                    held.get(relative.parent().ok_or_else(invalid)?)
                        .ok_or_else(invalid)?,
                    relative.file_name().ok_or_else(invalid)?,
                    flags(),
                    Mode::empty(),
                )
            }
            .map(File::from)
            .map_err(|_| invalid())?;
            if node(&current)? != node(file)? {
                return Err(invalid());
            }
        }
        if node(&open_parent(root)?)? != self.parent {
            return Err(invalid());
        }
        crate::core::deadline::check(deadline)
    }
}

fn verify_held(
    root: &Path,
    parent: &File,
    expected_parent: &Node,
    held: &BTreeMap<PathBuf, File>,
    deadline: Instant,
) -> Result<()> {
    if node(&open_parent(root)?)? != *expected_parent || node(parent)? != *expected_parent {
        return Err(invalid());
    }
    for (relative, original) in held {
        crate::core::deadline::check(deadline)?;
        let (directory, name) = if relative.as_os_str().is_empty() {
            (parent, root.file_name().ok_or_else(invalid)?)
        } else {
            (
                held.get(relative.parent().ok_or_else(invalid)?)
                    .ok_or_else(invalid)?,
                relative.file_name().ok_or_else(invalid)?,
            )
        };
        let current = File::from(
            rustix::fs::openat(directory, name, flags(), Mode::empty()).map_err(|_| invalid())?,
        );
        if node(&current)? != node(original)? {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(super) fn create(
    root: &Path,
    device: u64,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Snapshot> {
    let parent = open_parent(root)?;
    let parent_node = node(&parent)?;
    if parent_node.device != device {
        return Err(invalid());
    }
    let mut held: BTreeMap<PathBuf, File> = BTreeMap::new();
    let mut nodes = Vec::new();
    for relative in paths() {
        crate::core::deadline::check(deadline)?;
        guard()?;
        verify_held(root, &parent, &parent_node, &held, deadline)?;
        let (directory, name) = if relative.as_os_str().is_empty() {
            (&parent, root.file_name().ok_or_else(invalid)?)
        } else {
            (
                held.get(relative.parent().ok_or_else(invalid)?)
                    .ok_or_else(invalid)?,
                relative.file_name().ok_or_else(invalid)?,
            )
        };
        node(directory)?;
        rustix::fs::mkdirat(directory, name, Mode::RWXU).map_err(|_| invalid())?;
        let file = File::from(
            rustix::fs::openat(directory, name, flags(), Mode::empty()).map_err(|_| invalid())?,
        );
        let identity = node(&file)?;
        if identity.device != device {
            return Err(invalid());
        }
        nodes.push(identity);
        held.insert(relative, file);
        guard()?;
        verify_held(root, &parent, &parent_node, &held, deadline)?;
    }
    for file in held.values().rev() {
        guard()?;
        verify_held(root, &parent, &parent_node, &held, deadline)?;
        file.sync_all().map_err(|_| invalid())?;
    }
    guard()?;
    verify_held(root, &parent, &parent_node, &held, deadline)?;
    parent.sync_all().map_err(|_| invalid())?;
    guard()?;
    let snapshot = Snapshot {
        parent: parent_node,
        nodes,
    };
    snapshot.verify(root, true, deadline)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    use std::time::Duration;

    #[test]
    fn fixed_identity_rejects_replacement_but_allows_later_contents() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().canonicalize().unwrap().join("runtimes");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&parent)
            .unwrap();
        let root = parent.join("run");
        let deadline = Instant::now() + Duration::from_secs(10);
        let device = std::fs::metadata(&parent).unwrap().dev();
        let snapshot = create(&root, device, deadline, &|| Ok(())).unwrap();
        snapshot.verify(&root, true, deadline).unwrap();
        std::fs::write(root.join("logs/output"), b"synthetic").unwrap();
        snapshot.verify(&root, false, deadline).unwrap();
        assert!(snapshot.verify(&root, true, deadline).is_err());
        assert!(create(&root, device, deadline, &|| Ok(())).is_err());
        std::fs::rename(root.join("temp"), root.join("old-temp")).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(root.join("temp"))
            .unwrap();
        assert!(snapshot.verify(&root, false, deadline).is_err());
    }

    #[test]
    fn private_mode_and_held_path_replacement_are_checked() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().canonicalize().unwrap().join("runtimes");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&parent)
            .unwrap();
        let root = parent.join("run");
        let deadline = Instant::now() + Duration::from_secs(10);
        let device = std::fs::metadata(&parent).unwrap().dev();
        let snapshot = create(&root, device, deadline, &|| Ok(())).unwrap();
        std::fs::set_permissions(root.join("logs"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        assert!(snapshot.verify(&root, false, deadline).is_err());
        std::fs::set_permissions(root.join("logs"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let original_parent = open_parent(&root).unwrap();
        let original_root = File::from(rustix::fs::open(&root, flags(), Mode::empty()).unwrap());
        let held = BTreeMap::from([(PathBuf::new(), original_root)]);
        std::fs::rename(&root, parent.join("old-run")).unwrap();
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&root)
            .unwrap();
        assert!(verify_held(&root, &original_parent, &snapshot.parent, &held, deadline).is_err());
    }
}

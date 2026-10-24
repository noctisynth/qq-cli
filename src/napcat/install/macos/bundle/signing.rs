//! Read-only, exhaustive signing target plan bound to one application inventory.
use super::*;
use crate::napcat::install::macos::signing::Policy;
use crate::napcat::qq::macos::macho::ResignPayload;
use core_foundation::url::CFURL;
use security_framework::os::macos::code_signing::{Flags, SecStaticCode};
use std::collections::BTreeSet;
use std::path::Component;

mod execute;
mod supervisor;
mod sync;
mod transition;
pub use supervisor::worker_entry as supervisor_entry;

/// Explicit review input from a fixed-version layout audit, not user claims or
/// an inferred privilege grant from a filename. Main must be the root bundle.
pub struct BundleRole {
    path: PathBuf,
    identifier: String,
    executable: String,
    policy: Policy,
}
impl BundleRole {
    pub fn new(path: &Path, identifier: &str, executable: &str, policy: Policy) -> Result<Self> {
        if path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || path.as_os_str().len() > 4096
            || path
                .to_str()
                .is_none_or(|s| s.chars().any(char::is_control))
            || identifier.is_empty()
            || identifier.len() > 255
            || !identifier
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || executable.is_empty()
            || executable.len() > 255
            || executable.contains(['/', '\\'])
            || executable == "."
            || executable == ".."
            || executable.chars().any(char::is_control)
            || (policy == Policy::Main) != path.as_os_str().is_empty()
        {
            return Err(invalid());
        }
        Ok(Self {
            path: path.into(),
            identifier: identifier.into(),
            executable: executable.into(),
            policy,
        })
    }
}

/// Private paths are deliberately not Debug/Serialize. Merely planning does
/// not create the live-copy write capability required by a future signer.
pub struct SigningPlan {
    root: PathBuf,
    fingerprint: String,
    targets: Vec<SigningTarget>,
}
pub struct SigningTarget {
    path: PathBuf,
    executable: PathBuf,
    policy: Policy,
    identifier: Option<String>,
    payload: ResignPayload,
    seal: Option<PathBuf>,
}
impl SigningTarget {
    pub fn relative_path(&self) -> &Path {
        &self.path
    }
    pub fn relative_executable(&self) -> &Path {
        &self.executable
    }
    pub fn policy(&self) -> Policy {
        self.policy
    }
    pub fn bundle_identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }
}
impl SigningPlan {
    /// Verify only preserved Mach-O payloads after signing. This does not accept
    /// resource/metadata changes, validate new signatures, or issue a receipt;
    /// the signing transaction must independently enforce those boundaries.
    pub fn verify_payloads(&self, inventory: &Inventory, deadline: Instant) -> Result<()> {
        if inventory.root != self.root {
            return Err(invalid());
        }
        inventory.revalidate(deadline)?;
        for target in &self.targets {
            let entry = inventory
                .entries
                .get(&target.executable)
                .ok_or_else(invalid)?;
            let Content::File { size, .. } = entry.content else {
                return Err(invalid());
            };
            let mut file = inventory.signing_file(&target.executable)?;
            if ResignPayload::inspect(&mut file, size, deadline)? != target.payload {
                return Err(invalid());
            }
        }
        inventory.revalidate(deadline)
    }

    /// Inside-out order; every native image appears exactly once, at its bundle
    /// root or as a standalone file. The root application is always last.
    pub fn targets(&self) -> &[SigningTarget] {
        &self.targets
    }
    pub fn revalidate(&self, inventory: &Inventory, deadline: Instant) -> Result<()> {
        if inventory.root != self.root || inventory.fingerprint != self.fingerprint {
            return Err(invalid());
        }
        inventory.revalidate(deadline)
    }
}

impl Inventory {
    pub fn signing_plan(&self, roles: &[BundleRole], deadline: Instant) -> Result<SigningPlan> {
        self.revalidate(deadline)?;
        if roles.is_empty() || roles.len() > 256 {
            return Err(invalid());
        }
        let mut bundles = BTreeSet::new();
        let mut images = BTreeMap::new();
        for (path, entry) in &self.entries {
            crate::core::deadline::check(deadline)?;
            if matches!(entry.content, Content::Directory)
                && (path.as_os_str().is_empty()
                    || path.extension().is_some_and(|s| {
                        ["app", "framework", "xpc", "appex", "bundle"]
                            .iter()
                            .any(|ext| s == *ext)
                    }))
            {
                bundles.insert(path.clone());
            }
            if let Content::File { size, .. } = entry.content {
                if size < 4 {
                    continue;
                }
                let mut file = self.signing_file(path)?;
                let mut magic = [0; 4];
                file.read_exact(&mut magic).map_err(|_| invalid())?;
                if !matches!(
                    magic,
                    [0xcf, 0xfa, 0xed, 0xfe]
                        | [0xfe, 0xed, 0xfa, 0xcf]
                        | [0xce, 0xfa, 0xed, 0xfe]
                        | [0xfe, 0xed, 0xfa, 0xce]
                        | [0xca, 0xfe, 0xba, 0xbe]
                        | [0xca, 0xfe, 0xba, 0xbf]
                ) {
                    continue;
                }
                let kind = crate::napcat::qq::macos::macho::arm64_kind(&mut file, size)?;
                if ![2, 6, 8].contains(&kind) {
                    return Err(invalid());
                }
                let payload = ResignPayload::inspect(&mut file, size, deadline)?;
                images.insert(path.clone(), (kind, payload));
            }
        }
        if images.is_empty() || roles.len() != bundles.len() {
            return Err(invalid());
        }
        let mut covered = BTreeSet::new();
        let mut targets = Vec::new();
        for role in roles {
            crate::core::deadline::check(deadline)?;
            if !bundles.remove(&role.path) {
                return Err(invalid());
            }
            let path = self.root.join(&role.path);
            let framework = role.path.extension().is_some_and(|e| e == "framework");
            let metadata = path
                .join(if framework {
                    "Resources/Info.plist"
                } else {
                    "Contents/Info.plist"
                })
                .canonicalize()
                .map_err(|_| invalid())?;
            if !metadata.starts_with(&path) {
                return Err(invalid());
            }
            let metadata = metadata.strip_prefix(&self.root).map_err(|_| invalid())?;
            if !matches!(self.entries.get(metadata).map(|e| &e.content), Some(Content::File { size, .. }) if *size <= 1024*1024)
            {
                return Err(invalid());
            }
            let mut input = self.signing_file(metadata)?;
            let mut bytes = Vec::new();
            std::io::Read::by_ref(&mut input)
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| invalid())?;
            let actual: [u8; 32] = Sha256::digest(&bytes).into();
            if !matches!(&self.entries[metadata].content, Content::File { size, hash } if *size == bytes.len() as u64 && *hash == actual)
                || Stamp::from(&input.metadata().map_err(|_| invalid())?) != self.stamps[metadata]
            {
                return Err(invalid());
            }
            let url = CFURL::from_path(&path, true).ok_or_else(invalid)?;
            let code = SecStaticCode::from_path(&url, Flags::NONE).map_err(|_| invalid())?;
            let (exe, identifier, name) = crate::native::macos_signing_info::bundle(&code, &bytes)?;
            let executable = exe.canonicalize().map_err(|_| invalid())?;
            if !executable.starts_with(&path)
                || identifier != role.identifier
                || name != role.executable
                || executable
                    .file_name()
                    .is_none_or(|n| n != role.executable.as_str())
            {
                return Err(invalid());
            }
            let relative = executable
                .strip_prefix(&self.root)
                .map_err(|_| invalid())?
                .to_path_buf();
            let metadata_parent = metadata.parent().ok_or_else(invalid)?;
            let seal = if framework {
                metadata_parent.parent().ok_or_else(invalid)?
            } else {
                metadata_parent
            }
            .join("_CodeSignature/CodeResources");
            let (kind, payload) = *images.get(&relative).ok_or_else(invalid)?;
            let application = role.path.as_os_str().is_empty()
                || role
                    .path
                    .extension()
                    .is_some_and(|e| e == "app" || e == "xpc" || e == "appex");
            if (application && kind != 2)
                || (framework && kind != 6)
                || (!application && !framework && ![6, 8].contains(&kind))
                || (!application && role.policy != Policy::Native)
                || !covered.insert(relative.clone())
            {
                return Err(invalid());
            }
            targets.push(SigningTarget {
                path: role.path.clone(),
                executable: relative,
                policy: role.policy,
                identifier: Some(role.identifier.clone()),
                payload,
                seal: Some(seal),
            });
        }
        for (path, (_, payload)) in images.iter().filter(|(path, _)| !covered.contains(*path)) {
            targets.push(SigningTarget {
                path: path.clone(),
                executable: path.clone(),
                policy: Policy::Native,
                identifier: None,
                payload: *payload,
                seal: None,
            });
        }
        targets.sort_by(|a, b| {
            b.path
                .components()
                .count()
                .cmp(&a.path.components().count())
                .then_with(|| a.path.cmp(&b.path))
        });
        if targets
            .last()
            .is_none_or(|t| !t.path.as_os_str().is_empty())
        {
            return Err(invalid());
        }
        self.revalidate(deadline)?;
        Ok(SigningPlan {
            root: self.root.clone(),
            fingerprint: self.fingerprint.clone(),
            targets,
        })
    }

    pub(super) fn signing_file(&self, relative: &Path) -> Result<File> {
        let mut file = open(&self.root, None, true, false)?;
        if Stamp::from(&file.metadata().map_err(|_| invalid())?) != self.stamps[Path::new("")] {
            return Err(invalid());
        }
        let mut path = PathBuf::new();
        for component in relative.components() {
            path.push(component);
            let entry = self.entries.get(&path).ok_or_else(invalid)?;
            file = open(
                &path,
                Some(&file),
                matches!(entry.content, Content::Directory),
                false,
            )?;
            if Stamp::from(&file.metadata().map_err(|_| invalid())?) != self.stamps[&path] {
                return Err(invalid());
            }
        }
        Ok(file)
    }
}

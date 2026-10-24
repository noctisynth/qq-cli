use super::{BRIDGE_SOURCE, ENTRY_SOURCE, Prepared};
use crate::core::error::Result;
use crate::storage::state::durable;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Deterministic original assets, before publication. No filesystem observation
/// or ownership is implied by possessing the expected digest and destination.
pub struct AdapterPlan {
    main: String,
    parent: PathBuf,
    digest: String,
    entry: PathBuf,
}

impl Prepared {
    /// Caller must hold the lifecycle lock and authenticate the installation.
    /// The guard rechecks that authority before and after publication. This
    /// publishes only original assets, never modifies QQ or starts a process.
    /// Existing destinations are not overwritten, even if their bytes match.
    pub fn publish_adapter(&self, guard: impl Fn() -> Result<()>) -> Result<PathBuf> {
        self.adapter_plan()?.publish(guard)
    }

    /// Pure planning: usable before the durable deployment intent and before
    /// the adapter parent exists. Publication and verification are separate.
    pub fn adapter_plan(&self) -> Result<AdapterPlan> {
        let main = self.main_source()?;
        let digest = digest(&files(&main));
        let parent = self.current_directory().join("adapter");
        let entry = parent.join(&digest).join("main.cjs");
        Ok(AdapterPlan {
            main,
            parent,
            digest,
            entry,
        })
    }

    /// Recompute from this binary and these paths; no manifest supplied by the
    /// directory can redefine expected bytes. Recheck again at the effect boundary.
    pub fn verify_adapter(&self) -> Result<PathBuf> {
        self.adapter_plan()?.verify()
    }
}

impl AdapterPlan {
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn entry(&self) -> &Path {
        &self.entry
    }
    pub fn publish(&self, guard: impl Fn() -> Result<()>) -> Result<PathBuf> {
        durable::publish_files_new(&self.parent, &self.digest, &files(&self.main), guard)
            .map(|directory| directory.join("main.cjs"))
    }
    pub fn verify(&self) -> Result<PathBuf> {
        durable::verify_files(&self.parent.join(&self.digest), &files(&self.main))?;
        Ok(self.entry.clone())
    }
}

fn files(main: &str) -> [(&str, &[u8]); 3] {
    [
        ("main.cjs", main.as_bytes()),
        ("entry.cjs", ENTRY_SOURCE.as_bytes()),
        ("bridge.cjs", BRIDGE_SOURCE.as_bytes()),
    ]
}

fn digest(files: &[(&str, &[u8])]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qq-cli-macos-adapter-v1\0");
    for (name, bytes) in files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    format!("{:x}", digest.finalize())
}

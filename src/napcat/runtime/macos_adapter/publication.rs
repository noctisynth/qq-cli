use super::{BRIDGE_SOURCE, ENTRY_SOURCE, Prepared};
use crate::core::error::Result;
use crate::storage::state::durable;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

impl Prepared {
    /// Caller must hold the lifecycle lock and authenticate the installation.
    /// The guard rechecks that authority before and after publication. This
    /// publishes only original assets, never modifies QQ or starts a process.
    /// Existing destinations are not overwritten, even if their bytes match.
    pub fn publish_adapter(&self, guard: impl Fn() -> Result<()>) -> Result<PathBuf> {
        let main = self.main_source()?;
        let files = files(&main);
        let parent = self.current_directory().join("adapter");
        durable::publish_files_new(&parent, &digest(&files), &files, guard)
            .map(|directory| directory.join("main.cjs"))
    }

    /// Recompute from this binary and these paths; no manifest supplied by the
    /// directory can redefine expected bytes. Recheck again at the effect boundary.
    pub fn verify_adapter(&self) -> Result<PathBuf> {
        let main = self.main_source()?;
        let files = files(&main);
        let directory = self
            .current_directory()
            .join("adapter")
            .join(digest(&files));
        durable::verify_files(&directory, &files)?;
        Ok(directory.join("main.cjs"))
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

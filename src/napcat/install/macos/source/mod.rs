//! Fixed original QQ source observation for deployment assembly, never ownership.
use super::bundle::{BundleRole, Inventory, SigningPlan};
use crate::core::error::{AppError, Result};
use crate::napcat::qq::{Application, macos};
use crate::native::macos_signature::Origin;
use std::path::Path;
use std::time::Instant;

pub mod image;
mod roles;

/// Reviewed website artifact. These pins are not a downloader or a mount receipt.
pub const DMG_URL: &str =
    "https://qqdl.gtimg.cn/qqfile/QQNTV2/9.9.36/release/837fcb63/QQ_7.0.2_260924_01.dmg";
pub const DMG_BYTES: u64 = 488_441_429;
pub const DMG_SHA256: &str = "d270b78f0b210532637076532a883dc8659851b6daa92451983985ab9f3216f0";
pub const QQ_VERSION: &str = "7.0.2-53644";
pub const QQ_BUILD: &str = "53644";
const SELECTED: &str = "f5fba882516dab67090f825675d5f5afc7e05436f22231cafad45c99bcce2b51";
// Inventory v4 includes original modes, flags, ACLs, attributes and relative links.
// Do not normalize unknown metadata merely to make a source match this baseline.
const TREE: &str = "b8087574a4f873ba4cb72cfc758ab57e47588f26ddf0a75e8778749799126b21";

/// Public software pins only; this does not authenticate a local source.
pub(crate) fn reviewed_digest() -> String {
    use sha2::{Digest, Sha256};
    let fields = (
        DMG_URL,
        DMG_BYTES,
        DMG_SHA256,
        QQ_VERSION,
        QQ_BUILD,
        SELECTED,
        TREE,
        roles::digest(),
    );
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&fields).expect("fixed software metadata"))
    )
}

/// Cannot be constructed from caller-supplied version strings or an Inventory alone.
/// The inventory has no owner capability; all future effects require revalidation.
pub struct Source {
    original: macos::Original,
    inventory: Inventory,
}
impl Source {
    pub fn inspect(root: &Path, deadline: Instant) -> Result<Self> {
        // Snapshot the entire tree before signature verification, then require the
        // same complete tree and identities afterwards. Never mix two observations.
        let inventory = Inventory::inspect(root, deadline)?;
        if inventory.fingerprint() != TREE {
            return Err(unsupported());
        }
        let original = macos::inspect_original(&root.join("Contents/MacOS/QQ"), deadline)?;
        validate(
            original.application(),
            original.origin(),
            inventory.fingerprint(),
        )?;
        inventory.revalidate(deadline)?;
        Ok(Self {
            original,
            inventory,
        })
    }

    pub fn application(&self) -> &Application {
        self.original.application()
    }

    pub fn inventory(&self) -> &Inventory {
        &self.inventory
    }

    /// Read only the original bounded package bytes through the authenticated
    /// inventory's descriptors, digest and identity checks. Not write authority.
    pub fn package_bytes(&self, deadline: Instant) -> Result<Vec<u8>> {
        self.inventory.package_bytes(deadline)
    }

    /// Fixed reviewed metadata, also used to plan the authenticated dedicated
    /// copy after its entry edit. It does not authenticate an arbitrary copy.
    pub fn signing_roles(&self) -> Result<Vec<BundleRole>> {
        roles::roles()
    }

    pub fn signing_roles_digest(&self) -> String {
        roles::digest()
    }

    /// Read-only exhaustive plan for this exact original source. A dedicated
    /// copy needs its own plan after hardening and the package edit.
    pub fn signing_plan(&self, deadline: Instant) -> Result<SigningPlan> {
        self.inventory
            .signing_plan(&self.signing_roles()?, deadline)
    }

    /// Rechecks content, metadata and original inode identities; no source changes
    /// are accepted after the initial static publisher verification.
    pub fn revalidate(&self, deadline: Instant) -> Result<()> {
        self.inventory.revalidate(deadline)
    }
}

fn validate(app: &Application, origin: Origin, tree: &str) -> Result<()> {
    if origin != Origin::TencentDeveloperId
        || app.version != QQ_VERSION
        || app.build != QQ_BUILD
        || app.architecture != "aarch64"
        || app.fingerprint != SELECTED
        || tree != TREE
    {
        return Err(unsupported());
    }
    Ok(())
}

fn unsupported() -> AppError {
    AppError::new(
        "QQ_SOURCE_UNSUPPORTED",
        "QQ source does not match the reviewed website application's version, publisher and complete original tree.",
        4,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_version_does_not_substitute_origin_payload_tree_or_architecture() {
        // Test only the selector, not a forgeable Source or signature substitute.
        let app = Application {
            id: String::new(),
            version: QQ_VERSION.into(),
            build: QQ_BUILD.into(),
            architecture: "aarch64".into(),
            fingerprint: SELECTED.into(),
            executable_sha256: String::new(),
            major_sha256: String::new(),
            entry_sha256: String::new(),
            metadata_sha256: String::new(),
            executable: Default::default(),
        };
        validate(&app, Origin::TencentDeveloperId, TREE).unwrap();
        assert!(validate(&app, Origin::AppleAppStore, TREE).is_err());
        assert!(validate(&app, Origin::TencentDeveloperId, &"0".repeat(64)).is_err());
        for field in ["version", "build", "architecture", "fingerprint"] {
            let mut changed = app.clone();
            match field {
                "version" => changed.version = "7.0.3-53644".into(),
                "build" => changed.build = "53645".into(),
                "architecture" => changed.architecture = "x86_64".into(),
                _ => changed.fingerprint = "0".repeat(64),
            }
            assert!(validate(&changed, Origin::TencentDeveloperId, TREE).is_err());
        }
    }
}

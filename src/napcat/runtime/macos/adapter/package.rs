//! Plan and deploy the sole QQ metadata change. No signing or process startup.
use super::Prepared;
use crate::core::error::{AppError, Result};
use serde::{Deserialize, Deserializer, de};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const ORIGINAL_MAIN: &str = "./application.asar/app_launcher/index.js";

/// The deployment caller authenticates the source digest/version against its
/// verified official DMG. A caller-supplied digest alone is not provenance.
/// No Debug/Serialize: the generated metadata contains a private local path.
pub struct PackageEdit {
    bytes: Vec<u8>,
    source_digest: String,
    result_digest: String,
    destination: PathBuf,
    entry: PathBuf,
}

impl Prepared {
    /// Electron joins package.main to QQ.app/Contents/Resources/app. An
    /// absolute main is still appended by that join, so use the fixed layout
    /// back to this run's external adapter instead.
    pub(super) fn package_main(&self) -> Result<String> {
        let adapter = self.adapter_plan()?;
        let relative = adapter
            .entry()
            .strip_prefix(self.current_directory())
            .map_err(|_| invalid())?;
        Path::new("../../../..")
            .join(relative)
            .to_str()
            .map(str::to_owned)
            .ok_or_else(invalid)
    }

    /// A failure consumes the copy capability and retains any partial edit.
    /// Success still requires signing and launch validation.
    pub fn deploy_package(
        &self,
        copy: crate::napcat::install::macos::bundle::Inventory,
        edit: &PackageEdit,
        deadline: std::time::Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<crate::napcat::install::macos::bundle::Inventory> {
        copy.deploy_package(
            edit.destination(),
            edit.source_digest(),
            edit.bytes(),
            deadline,
            || {
                guard()?;
                if self.verify_adapter()? != edit.entry
                    || edit.destination
                        != self
                            .current_directory()
                            .join("QQ.app/Contents/Resources/app/package.json")
                {
                    return Err(invalid());
                }
                Ok(())
            },
        )
    }

    pub fn package_edit(
        &self,
        original: &[u8],
        expected_digest: &str,
        expected_version: &str,
        expected_build: &str,
    ) -> Result<PackageEdit> {
        let edit =
            self.plan_package_edit(original, expected_digest, expected_version, expected_build)?;
        self.verify_adapter()?;
        Ok(edit)
    }

    /// Read authenticated source metadata without publishing any adapter assets.
    /// The resulting edit still requires live-copy authority and effect checks.
    pub fn plan_source_package(
        &self,
        source: &crate::napcat::install::macos::source::Source,
        deadline: std::time::Instant,
    ) -> Result<PackageEdit> {
        let bytes = source.package_bytes(deadline)?;
        let app = source.application();
        self.plan_package_edit(&bytes, &hex(&bytes), &app.version, &app.build)
    }

    /// Pure byte planning, not source authentication or proof of publication.
    /// Production assembly uses plan_source_package with an authenticated Source.
    pub fn plan_package_edit(
        &self,
        original: &[u8],
        expected_digest: &str,
        expected_version: &str,
        expected_build: &str,
    ) -> Result<PackageEdit> {
        if original.len() > 1024 * 1024
            || expected_digest.len() != 64
            || !expected_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || hex(original) != expected_digest
            || expected_version.is_empty()
            || expected_build.is_empty()
        {
            return Err(invalid());
        }
        let Object(mut object) = serde_json::from_slice(original).map_err(|_| invalid())?;
        for (key, expected) in [
            ("name", "qq-chat"),
            ("platform", "darwin"),
            ("version", expected_version),
            ("buildVersion", expected_build),
            ("main", ORIGINAL_MAIN),
        ] {
            if object.get(key).and_then(Value::as_str) != Some(expected) {
                return Err(invalid());
            }
        }
        if object
            .get("type")
            .is_some_and(|kind| kind.as_str() != Some("commonjs"))
        {
            return Err(invalid());
        }
        // Electron package eleArch is not a Mach-O architecture witness. The
        // dedicated copy must be independently checked for native arm64.
        let adapter = self.adapter_plan()?;
        let entry = adapter.entry();
        object.insert("main".into(), Value::String(self.package_main()?));
        let mut bytes = serde_json::to_vec_pretty(&object).map_err(|_| invalid())?;
        bytes.push(b'\n');
        Ok(PackageEdit {
            entry: entry.to_owned(),
            result_digest: hex(&bytes),
            bytes,
            source_digest: expected_digest.into(),
            destination: self
                .current_directory()
                .join("QQ.app/Contents/Resources/app/package.json"),
        })
    }
}

struct Object(serde_json::Map<String, Value>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Object;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an object with unique package fields")
            }
            fn visit_map<M: de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Object, M::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, Value>()? {
                    if values.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate package field"));
                    }
                }
                Ok(Object(values))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

impl PackageEdit {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn source_digest(&self) -> &str {
        &self.source_digest
    }
    pub fn result_digest(&self) -> &str {
        &self.result_digest
    }
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Use before replacing the dedicated copy's original metadata. Do not use
    /// this to authorize writes to the ordinary installation or skip its seal.
    pub fn verify_source(&self, bytes: &[u8]) -> Result<()> {
        if hex(bytes) != self.source_digest {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn verify_result(&self, bytes: &[u8]) -> Result<()> {
        if bytes != self.bytes {
            return Err(invalid());
        }
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn invalid() -> AppError {
    AppError::new(
        "MACOS_PACKAGE_CONFLICT",
        "QQ package metadata or prepared entry changed; no successful deployment was accepted.",
        4,
    )
}

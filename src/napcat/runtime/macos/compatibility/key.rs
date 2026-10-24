//! Exact software/environment identity, independent of instance paths and secrets.
use super::*;
use crate::napcat::install::macos::source;
use crate::napcat::runtime::macos::release;
use crate::napcat::runtime::macos::{adapter, start::sandbox};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Key {
    schema_version: u8,
    architecture: String,
    kernel_release: String,
    kernel_version: String,
    system_version_sha256: String,
    framework_version: String,
    commit: String,
    archive_sha256: String,
    qq_source_sha256: String,
    launch_software_sha256: String,
}
impl Key {
    pub fn current() -> Result<Self> {
        Self::for_version(release::DEFAULT_VERSION)
    }
    pub fn for_version(version: &str) -> Result<Self> {
        let os = rustix::system::uname();
        if os.sysname().to_bytes() != b"Darwin" || os.machine().to_bytes() != b"arm64" {
            return Err(unverified());
        }
        let release = release::lookup(version)?;
        let software = serde_json::to_vec(&(
            adapter::Prepared::software_descriptor()?,
            sandbox::profile(Path::new("/qq-cli-runtime"), Path::new("/qq-cli-framework"))?,
            configuration_descriptor(release.version)?,
        ))
        .map_err(|_| unverified())?;
        let key = Self {
            schema_version: 1,
            architecture: "aarch64".into(),
            kernel_release: os.release().to_str().map_err(|_| unverified())?.into(),
            kernel_version: os.version().to_str().map_err(|_| unverified())?.into(),
            system_version_sha256: crate::native::platform::macos::system_version_digest()?,
            framework_version: release.version.into(),
            commit: release.commit.into(),
            archive_sha256: release.archive.sha256.into(),
            qq_source_sha256: source::reviewed_digest(),
            launch_software_sha256: format!("{:x}", Sha256::digest(software)),
        };
        key.validate()?;
        Ok(key)
    }
    pub fn fingerprint(&self) -> Result<String> {
        self.validate()?;
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).map_err(|_| unverified())?)
        ))
    }
    pub(super) fn archive(&self) -> &str {
        &self.archive_sha256
    }
    pub(in crate::napcat::runtime::macos) fn validate(&self) -> Result<()> {
        let text = |s: &str| !s.is_empty() && s.len() <= 512 && !s.chars().any(char::is_control);
        let hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.schema_version != 1
            || self.architecture != "aarch64"
            || !text(&self.kernel_release)
            || !text(&self.kernel_version)
            || release::lookup(&self.framework_version).is_err()
            || self.commit.len() != 40
            || !self.commit.bytes().all(|b| b.is_ascii_hexdigit())
            || ![
                &self.system_version_sha256,
                &self.archive_sha256,
                &self.qq_source_sha256,
                &self.launch_software_sha256,
            ]
            .into_iter()
            .all(|s| hash(s))
        {
            return Err(unverified());
        }
        Ok(())
    }
}

fn configuration_descriptor(version: &str) -> Result<Vec<u8>> {
    use crate::napcat::runtime::runtime_config::{Bundle, Ports};
    // Fixed placeholders, not generated credentials or a running instance.
    let bundle = Bundle::restore(
        version,
        Ports {
            http: 16001,
            events: 16002,
            control: 16003,
        },
        "0".repeat(64),
        "1".repeat(64),
        "2".repeat(64),
    )?;
    let mut files = Vec::new();
    for file in bundle.files() {
        let mut bytes = Vec::new();
        file.write_to(&mut bytes)?;
        files.push((file.name(), format!("{:x}", Sha256::digest(bytes))));
    }
    let mut environment = Vec::new();
    bundle.visit_environment(|name, value| {
        environment.push((name.to_owned(), value.to_owned()));
        Ok(())
    })?;
    serde_json::to_vec(&(files, environment)).map_err(|_| unverified())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_software_key_contains_no_host_name_paths_or_secret_and_changes_with_each_input() {
        let key = Key::current().unwrap();
        assert_eq!(
            key.fingerprint().unwrap(),
            Key::current().unwrap().fingerprint().unwrap()
        );
        let serialized = serde_json::to_string(&key).unwrap();
        assert!(!serialized.contains("/Users/"));
        assert!(!serialized.contains("nodename"));
        let fields = [
            "kernelRelease",
            "kernelVersion",
            "systemVersionSha256",
            "archiveSha256",
            "qqSourceSha256",
            "launchSoftwareSha256",
        ];
        let original = key.fingerprint().unwrap();
        for field in fields {
            let mut json = serde_json::to_value(&key).unwrap();
            json[field] = if field.ends_with("Sha256") {
                "0".repeat(64).into()
            } else {
                "different software".into()
            };
            let changed: Key = serde_json::from_value(json).unwrap();
            assert_ne!(changed.fingerprint().unwrap(), original);
        }
    }
}

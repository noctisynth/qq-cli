use super::*;
use crate::napcat::install::macos::source;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Serialize)]
struct Inputs<'a> {
    instance: &'a Instance,
    installation: &'a Installation,
    source: &'a crate::napcat::qq::Application,
    source_executable: &'a Path,
    source_tree: &'a str,
    source_dmg: (&'a str, u64, &'a str),
    framework_provenance: (&'a str, &'a str, &'a str),
    layout: (&'a Path, &'a str),
    roles: String,
    adapter: (&'a str, &'a Path),
    package: (&'a str, &'a str, &'a Path),
    working_directory: &'a Path,
    executable: PathBuf,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
}
pub(super) fn digest(
    source: &Source,
    instance: &Instance,
    installation: &Installation,
    prepared: &Prepared,
    adapter: &AdapterPlan,
    package: &PackageEdit,
    layout: &super::super::layout::Layout,
) -> Result<String> {
    let release = catalog::lookup("4.18.41")?;
    let text = |value: &std::ffi::OsStr| value.to_str().map(str::to_owned).ok_or_else(invalid);
    let inputs = Inputs {
        instance,
        installation,
        source: source.application(),
        source_executable: &source.application().executable,
        source_tree: source.inventory().fingerprint(),
        source_dmg: (source::DMG_URL, source::DMG_BYTES, source::DMG_SHA256),
        framework_provenance: (release.version, release.commit, release.url),
        layout: (layout.directory(), layout.fingerprint()),
        roles: source.signing_roles_digest(),
        adapter: (adapter.digest(), adapter.entry()),
        package: (
            package.source_digest(),
            package.result_digest(),
            package.destination(),
        ),
        working_directory: prepared.current_directory(),
        executable: prepared.executable(),
        arguments: prepared
            .arguments()
            .iter()
            .map(|v| text(v))
            .collect::<Result<_>>()?,
        environment: prepared
            .environment()
            .iter()
            .map(|(k, v)| Ok((text(k)?, text(v)?)))
            .collect::<Result<_>>()?,
    };
    hash(&inputs)
}

fn hash(inputs: &Inputs<'_>) -> Result<String> {
    let bytes = serde_json::to_vec(inputs).map_err(|_| invalid())?;
    let mut hash = Sha256::new();
    hash.update(b"qq-cli-macos-deployment-input-v2\0");
    hash.update(bytes);
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_binds_scope_provenance_assets_roles_and_literal_launch_inputs() {
        let instance = Instance {
            id: "instance".into(),
            installation_id: "install".into(),
            endpoint: "http://127.0.0.1:12345".into(),
            ws_endpoint: None,
            credential_key: "synthetic".into(),
            account: None,
            run_id: "run".into(),
            state: "prepared".into(),
            process_id: None,
            process_started: None,
        };
        let installation = Installation {
            id: "install".into(),
            version: "fixture".into(),
            digest: "a".repeat(64),
            directory: PathBuf::from("framework"),
        };
        let source = crate::napcat::qq::Application {
            id: "source".into(),
            version: "fixture".into(),
            build: "1".into(),
            architecture: "aarch64".into(),
            fingerprint: "b".repeat(64),
            executable_sha256: "c".repeat(64),
            major_sha256: "d".repeat(64),
            entry_sha256: "e".repeat(64),
            metadata_sha256: "f".repeat(64),
            executable: PathBuf::from("original/QQ"),
        };
        let mut inputs = Inputs {
            instance: &instance,
            installation: &installation,
            source: &source,
            source_executable: &source.executable,
            source_tree: "tree",
            source_dmg: ("url", 123, "dmg"),
            framework_provenance: ("version", "commit", "url"),
            layout: (Path::new("runtime"), "receipt"),
            roles: "roles".into(),
            adapter: ("adapter", Path::new("entry")),
            package: ("before", "after", Path::new("package")),
            working_directory: Path::new("work"),
            executable: PathBuf::from("copy/QQ"),
            arguments: vec!["literal".into()],
            environment: BTreeMap::from([("A".into(), "1".into()), ("B".into(), "2".into())]),
        };
        let baseline = hash(&inputs).unwrap();
        inputs.environment = BTreeMap::from([("B".into(), "2".into()), ("A".into(), "1".into())]);
        assert_eq!(hash(&inputs).unwrap(), baseline);
        macro_rules! changed {
            ($field:ident, $value:expr) => {{
                let old = std::mem::replace(&mut inputs.$field, $value);
                assert_ne!(hash(&inputs).unwrap(), baseline, stringify!($field));
                inputs.$field = old;
            }};
        }
        let mut other_instance = instance.clone();
        other_instance.run_id = "other-run".into();
        let mut other_installation = installation.clone();
        other_installation.directory = "other-framework".into();
        let mut other_source = source.clone();
        other_source.fingerprint = "0".repeat(64);
        changed!(instance, &other_instance);
        changed!(installation, &other_installation);
        changed!(source, &other_source);
        changed!(source_executable, Path::new("other-source/QQ"));
        changed!(source_tree, "other-tree");
        changed!(source_dmg, ("url", 124, "dmg"));
        changed!(framework_provenance, ("version", "other-commit", "url"));
        changed!(layout, (Path::new("runtime"), "other-receipt"));
        changed!(layout, (Path::new("other-runtime"), "receipt"));
        changed!(roles, "other-roles".into());
        changed!(adapter, ("other-adapter", Path::new("entry")));
        changed!(package, ("before", "other-result", Path::new("package")));
        changed!(working_directory, Path::new("other-work"));
        changed!(executable, PathBuf::from("other-copy/QQ"));
        changed!(arguments, vec!["other-literal".into()]);
        changed!(
            environment,
            BTreeMap::from([("A".into(), "other-value".into())])
        );
    }
}

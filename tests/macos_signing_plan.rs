#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::napcat::install::macos_bundle::{BundleRole, Inventory};
use qq_cli::napcat::install::macos_signing::Policy;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(30)
}
fn plist(path: &Path, name: &str, id: &str, kind: &str) {
    std::fs::write(path,format!(r#"<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>{id}</string><key>CFBundleExecutable</key><string>{name}</string><key>CFBundlePackageType</key><string>{kind}</string><key>CFBundleVersion</key><string>1</string></dict></plist>"#)).unwrap();
}
fn app(root: &Path, name: &str, id: &str) {
    std::fs::create_dir_all(root.join("Contents/MacOS")).unwrap();
    std::fs::create_dir_all(root.join("Contents/Resources")).unwrap();
    std::fs::copy(
        std::env::current_exe().unwrap(),
        root.join("Contents/MacOS").join(name),
    )
    .unwrap();
    plist(&root.join("Contents/Info.plist"), name, id, "APPL");
}
fn fixture(base: &Path) -> PathBuf {
    let root = base.join("Synthetic.app");
    app(&root, "Main", "org.qq-cli.synthetic");
    app(
        &root.join("Contents/Frameworks/Worker.app"),
        "Worker",
        "org.qq-cli.worker",
    );
    let framework = root.join("Contents/Frameworks/Thing.framework");
    let version = framework.join("Versions/A");
    std::fs::create_dir_all(version.join("Resources")).unwrap();
    plist(
        &version.join("Resources/Info.plist"),
        "Thing",
        "org.qq-cli.thing",
        "FMWK",
    );
    let source = base.join("fixture.c");
    std::fs::write(&source, "int synthetic(void) { return 7; }\n").unwrap();
    assert!(
        std::process::Command::new("clang")
            .args(["-dynamiclib", "-arch", "arm64"])
            .arg(&source)
            .arg("-o")
            .arg(version.join("Thing"))
            .output()
            .unwrap()
            .status
            .success()
    );
    std::fs::copy(
        version.join("Thing"),
        root.join("Contents/Resources/plugin.node"),
    )
    .unwrap();
    std::os::unix::fs::symlink("A", framework.join("Versions/Current")).unwrap();
    std::os::unix::fs::symlink("Versions/Current/Thing", framework.join("Thing")).unwrap();
    std::os::unix::fs::symlink("Versions/Current/Resources", framework.join("Resources")).unwrap();
    assert!(
        std::process::Command::new("/usr/bin/codesign")
            .args(["--force", "--deep", "--sign", "-", "--timestamp=none"])
            .arg(&root)
            .output()
            .unwrap()
            .status
            .success()
    );
    root
}
fn roles() -> Vec<BundleRole> {
    vec![
        BundleRole::new(Path::new(""), "org.qq-cli.synthetic", "Main", Policy::Main).unwrap(),
        BundleRole::new(
            Path::new("Contents/Frameworks/Worker.app"),
            "org.qq-cli.worker",
            "Worker",
            Policy::Electron,
        )
        .unwrap(),
        BundleRole::new(
            Path::new("Contents/Frameworks/Thing.framework"),
            "org.qq-cli.thing",
            "Thing",
            Policy::Native,
        )
        .unwrap(),
    ]
}

#[test]
fn exhaustive_plan_orders_images_inside_out_and_binds_original_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    // Entry deployment invalidates the old seal before resigning. Metadata
    // planning is still possible; this is deliberately not source validation.
    std::fs::write(
        root.join("Contents/Resources/entry-change"),
        "synthetic entry",
    )
    .unwrap();
    let inventory = Inventory::inspect(&root, deadline()).unwrap();
    let plan = inventory.signing_plan(&roles(), deadline()).unwrap();
    let targets = plan.targets();
    assert_eq!(targets.len(), 4);
    assert_eq!(targets.last().unwrap().relative_path(), Path::new(""));
    assert!(targets.last().unwrap().policy() == Policy::Main);
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.relative_executable()
                == Path::new("Contents/Frameworks/Thing.framework/Versions/A/Thing"))
            .count(),
        1
    );
    assert!(targets.iter().any(|t| t.relative_path()
        == Path::new("Contents/Resources/plugin.node")
        && t.policy() == Policy::Native));
    plan.revalidate(&inventory, deadline()).unwrap();
    std::fs::write(root.join("Contents/Resources/changed"), "changed").unwrap();
    assert!(plan.revalidate(&inventory, deadline()).is_err());
    let changed = Inventory::inspect(&root, deadline()).unwrap();
    assert!(plan.revalidate(&changed, deadline()).is_err());
}

#[test]
fn incomplete_duplicate_wrong_metadata_and_unknown_bundles_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    let inventory = Inventory::inspect(&root, deadline()).unwrap();
    let mut missing = roles();
    missing.pop();
    assert!(inventory.signing_plan(&missing, deadline()).is_err());
    let mut duplicate = roles();
    duplicate[2] =
        BundleRole::new(Path::new(""), "org.qq-cli.synthetic", "Main", Policy::Main).unwrap();
    assert!(inventory.signing_plan(&duplicate, deadline()).is_err());
    for (id, name) in [("wrong", "Main"), ("org.qq-cli.synthetic", "Wrong")] {
        let mut wrong = roles();
        wrong[0] = BundleRole::new(Path::new(""), id, name, Policy::Main).unwrap();
        assert!(inventory.signing_plan(&wrong, deadline()).is_err());
    }
    app(
        &root.join("Contents/Resources/Unknown.app"),
        "Unknown",
        "org.qq-cli.unknown",
    );
    let changed = Inventory::inspect(&root, deadline()).unwrap();
    assert!(changed.signing_plan(&roles(), deadline()).is_err());
    assert!(changed.signing_plan(&roles(), Instant::now()).is_err());
}

#[test]
fn role_inputs_never_infer_main_privileges_from_nested_names() {
    for path in ["/outside.app", "../escape.app", "./alias.app"] {
        assert!(
            BundleRole::new(Path::new(path), "org.qq-cli.test", "Main", Policy::Electron).is_err()
        );
    }
    assert!(
        BundleRole::new(
            Path::new("Nested.app"),
            "org.qq-cli.test",
            "Main",
            Policy::Main
        )
        .is_err()
    );
    assert!(BundleRole::new(Path::new(""), "org.qq-cli.test", "Main", Policy::Native).is_err());
    for executable in ["../Main", "A/B", "", ".."] {
        assert!(
            BundleRole::new(Path::new(""), "org.qq-cli.test", executable, Policy::Main).is_err()
        );
    }
}

#[test]
fn bounded_inventory_metadata_does_not_require_optional_secured_plist() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("Unsealed.app");
    app(&root, "Main", "org.qq-cli.synthetic");
    let roles =
        [BundleRole::new(Path::new(""), "org.qq-cli.synthetic", "Main", Policy::Main).unwrap()];
    let inventory = Inventory::inspect(&root, deadline()).unwrap();
    assert_eq!(
        inventory
            .signing_plan(&roles, deadline())
            .unwrap()
            .targets()
            .len(),
        1
    );
    let info = root.join("Contents/Info.plist");
    std::fs::write(&info, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let oversized = Inventory::inspect(&root, deadline()).unwrap();
    assert!(oversized.signing_plan(&roles, deadline()).is_err());
    assert!(inventory.signing_plan(&roles, deadline()).is_err());
}

fn resign(path: &Path, policy: Policy, base: &Path) {
    let entitlements = base.join("resign-entitlements.plist");
    std::fs::write(&entitlements, policy.entitlements()).unwrap();
    assert!(
        std::process::Command::new("/usr/bin/codesign")
            .args([
                "--force",
                "--sign",
                "-",
                "--timestamp=none",
                "--options",
                "runtime",
                "--generate-entitlement-der",
                "--entitlements"
            ])
            .arg(&entitlements)
            .arg(path)
            .output()
            .unwrap()
            .status
            .success()
    );
    policy.verify(path, deadline()).unwrap();
}

#[test]
fn resigning_nested_synthetic_code_preserves_payload_but_new_valid_tampered_signature_does_not() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    let original = Inventory::inspect(&root, deadline()).unwrap();
    let plan = original.signing_plan(&roles(), deadline()).unwrap();
    for target in plan.targets() {
        resign(&root.join(target.relative_path()), target.policy(), &base);
    }
    let signed = Inventory::inspect(&root, deadline()).unwrap();
    assert!(plan.revalidate(&signed, deadline()).is_err());
    plan.verify_payloads(&signed, deadline()).unwrap();
    plan.verify_transition(&original, &signed, deadline())
        .unwrap();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("Contents/MacOS/Main"))
        .unwrap();
    file.seek(SeekFrom::Start(16384)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 1;
    file.seek(SeekFrom::Start(16384)).unwrap();
    file.write_all(&byte).unwrap();
    drop(file);
    resign(&root, Policy::Main, &base);
    let changed = Inventory::inspect(&root, deadline()).unwrap();
    assert!(plan.verify_payloads(&changed, deadline()).is_err());
}

#[test]
fn universal_resigning_preserves_both_slices_and_detects_foreign_code_changes() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("Universal.app");
    app(&root, "Main", "org.qq-cli.synthetic");
    let source = base.join("universal.c");
    std::fs::write(&source, "int main(void) { return 7; }\n").unwrap();
    let main = root.join("Contents/MacOS/Main");
    assert!(
        std::process::Command::new("clang")
            .args(["-arch", "arm64", "-arch", "x86_64"])
            .arg(&source)
            .arg("-o")
            .arg(&main)
            .output()
            .unwrap()
            .status
            .success()
    );
    resign(&root, Policy::Main, &base);
    let roles =
        [BundleRole::new(Path::new(""), "org.qq-cli.synthetic", "Main", Policy::Main).unwrap()];
    let inventory = Inventory::inspect(&root, deadline()).unwrap();
    let plan = inventory.signing_plan(&roles, deadline()).unwrap();
    resign(&root, Policy::Electron, &base);
    let signed = Inventory::inspect(&root, deadline()).unwrap();
    plan.verify_payloads(&signed, deadline()).unwrap();
    plan.verify_transition(&inventory, &signed, deadline())
        .unwrap();
    // No foreign code is executed. Change only the x86_64 __text bytes and
    // produce an otherwise valid signature over the changed universal image.
    let mut bytes = std::fs::read(&main).unwrap();
    let be = |b: &[u8]| u32::from_be_bytes(b.try_into().unwrap()) as usize;
    let le = |b: &[u8]| u32::from_le_bytes(b.try_into().unwrap()) as usize;
    assert_eq!(&bytes[..4], &[0xca, 0xfe, 0xba, 0xbe]);
    let slice = (0..be(&bytes[4..8]))
        .find_map(|i| {
            let at = 8 + i * 20;
            (be(&bytes[at..at + 4]) == 0x01000007).then(|| be(&bytes[at + 8..at + 12]))
        })
        .unwrap();
    let mut at = slice + 32;
    let mut code = None;
    for _ in 0..le(&bytes[slice + 16..slice + 20]) {
        let cmd = le(&bytes[at..at + 4]);
        let size = le(&bytes[at + 4..at + 8]);
        if cmd == 0x19 {
            for i in 0..le(&bytes[at + 64..at + 68]) {
                let section = at + 72 + i * 80;
                if &bytes[section..section + 16] == b"__text\0\0\0\0\0\0\0\0\0\0" {
                    code = Some(slice + le(&bytes[section + 48..section + 52]));
                }
            }
        }
        at += size;
    }
    bytes[code.unwrap()] ^= 1;
    std::fs::write(&main, bytes).unwrap();
    resign(&root, Policy::Electron, &base);
    let changed = Inventory::inspect(&root, deadline()).unwrap();
    assert!(plan.verify_payloads(&changed, deadline()).is_err());
}

#[test]
fn signed_tree_transition_rejects_resource_metadata_and_path_changes() {
    use std::os::unix::fs::PermissionsExt;
    for change in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = fixture(&base);
        let resource = root.join("Contents/Resources/data");
        std::fs::write(&resource, "original").unwrap();
        let original = Inventory::inspect(&root, deadline()).unwrap();
        let plan = original.signing_plan(&roles(), deadline()).unwrap();
        match change {
            0 => std::fs::write(&resource, "modified").unwrap(),
            1 => std::fs::write(root.join("Contents/Resources/extra"), "added").unwrap(),
            2 => std::fs::remove_file(&resource).unwrap(),
            3 => {
                let path = root.join("Contents/MacOS/Main");
                let mode = std::fs::metadata(&path).unwrap().permissions().mode();
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode ^ 0o040))
                    .unwrap();
            }
            4 => {
                let link = root.join("Contents/Frameworks/Thing.framework/Thing");
                std::fs::remove_file(&link).unwrap();
                std::os::unix::fs::symlink("Versions/A/Thing", link).unwrap();
            }
            _ => unreachable!(),
        }
        for target in plan.targets() {
            resign(&root.join(target.relative_path()), target.policy(), &base);
        }
        let changed = Inventory::inspect(&root, deadline()).unwrap();
        // All signatures are valid and native payloads unchanged, but that
        // cannot authorize unrelated resources, permissions or link changes.
        plan.verify_payloads(&changed, deadline()).unwrap();
        assert!(
            plan.verify_transition(&original, &changed, deadline())
                .is_err()
        );
    }
}

#[test]
fn signing_transaction_requires_live_hardened_copy_and_preserves_source() {
    use std::os::unix::fs::DirBuilderExt;
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    let original = Inventory::inspect(&root, deadline()).unwrap();
    let plan = original.signing_plan(&roles(), deadline()).unwrap();
    assert!(original.sign_copy(&plan, deadline(), || Ok(())).is_err());
    let original = Inventory::inspect(&root, deadline()).unwrap();
    let parent = base.join("private");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&parent)
        .unwrap();
    let copy = original
        .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
        .unwrap()
        .harden_copy(deadline(), || Ok(()))
        .unwrap();
    let plan = copy.signing_plan(&roles(), deadline()).unwrap();
    let signed = copy.sign_copy(&plan, deadline(), || Ok(())).unwrap();
    signed.revalidate(deadline()).unwrap();
    for target in plan.targets() {
        target
            .policy()
            .verify(
                &parent.join("QQ.app").join(target.relative_path()),
                deadline(),
            )
            .unwrap();
    }
    original.revalidate(deadline()).unwrap();
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 1);
}

#[test]
fn each_signing_step_rejects_changes_to_another_targets_seal() {
    use std::{
        cell::Cell,
        os::unix::fs::{DirBuilderExt, MetadataExt},
    };
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = fixture(&base);
    let original = Inventory::inspect(&root, deadline()).unwrap();
    let parent = base.join("private");
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&parent)
        .unwrap();
    let copy = original
        .copy_to_new(&parent, "QQ.app", deadline(), || Ok(()))
        .unwrap()
        .harden_copy(deadline(), || Ok(()))
        .unwrap();
    let plan = copy.signing_plan(&roles(), deadline()).unwrap();
    let first = parent
        .join("QQ.app")
        .join(plan.targets()[0].relative_executable());
    let inode = std::fs::metadata(&first).unwrap().ino();
    let changed = Cell::new(false);
    let result = copy.sign_copy(&plan, deadline(), || {
        if !changed.get() && std::fs::metadata(&first).unwrap().ino() != inode {
            changed.set(true);
            // Root signing later would overwrite this. It must be rejected at
            // this earlier target's boundary rather than silently repaired.
            std::fs::write(
                parent.join("QQ.app/Contents/_CodeSignature/CodeResources"),
                b"unrelated seal mutation",
            )
            .unwrap();
        }
        Ok(())
    });
    assert!(changed.get());
    assert!(result.is_err());
    original.revalidate(deadline()).unwrap();
}

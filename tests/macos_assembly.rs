#![cfg(all(target_os = "macos", target_arch = "aarch64"))]
use qq_cli::{
    core::error::AppError,
    napcat::{
        install::macos_bundle::Inventory,
        runtime::{macos_adapter::Prepared, macos_assembly::Assembly},
    },
};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn layout(root: &Path, framework: &Path) -> Prepared {
    std::fs::create_dir_all(framework).unwrap();
    for leaf in [
        "Desktop",
        "Documents",
        "Downloads",
        "Music",
        "Pictures",
        "Movies",
        "Library/Application Support/QQ",
    ] {
        std::fs::create_dir_all(
            root.join("profile-home/Library/Containers/com.tencent.qq/Data")
                .join(leaf),
        )
        .unwrap();
    }
    for leaf in [
        "temp",
        "logs",
        "config",
        "cache",
        "napcat-data/config",
        "napcat-data/logs",
        "napcat-data/cache",
        "napcat-data/plugins",
    ] {
        std::fs::create_dir_all(root.join(leaf)).unwrap();
    }
    Prepared::inspect(root, framework).unwrap()
}

#[test]
fn assembly_rejects_lost_authority_and_cross_plan_paths_before_any_publication() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let root = base.join("instance");
    let prepared = layout(&root, &base.join("framework"));
    let other = layout(&base.join("other"), &base.join("other-framework"));
    let adapter = prepared.adapter_plan().unwrap();
    let foreign_adapter = other.adapter_plan().unwrap();
    let bytes = br#"{"name":"qq-chat","platform":"darwin","version":"fixture","buildVersion":"1","main":"./application.asar/app_launcher/index.js"}"#;
    let digest = format!("{:x}", Sha256::digest(bytes));
    let package = prepared
        .plan_package_edit(bytes, &digest, "fixture", "1")
        .unwrap();
    let foreign_package = other
        .plan_package_edit(bytes, &digest, "fixture", "1")
        .unwrap();
    let source = base.join("Original.app");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("resource"), b"synthetic").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let source = Inventory::inspect(&source, deadline).unwrap();
    for variant in 0..4 {
        let result = Assembly {
            source: &source,
            roles: &[],
            prepared: &prepared,
            adapter: if variant == 2 {
                &foreign_adapter
            } else {
                &adapter
            },
            package: if variant == 3 {
                &foreign_package
            } else {
                &package
            },
        }
        .execute(
            &root.join(if variant == 1 { "Other.app" } else { "QQ.app" }),
            deadline,
            || {
                if variant == 0 {
                    Err(AppError::new("GUARD_LOST", "Synthetic guard loss", 3))
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            result.err().unwrap().code,
            if variant == 0 {
                "GUARD_LOST"
            } else {
                "MACOS_ASSEMBLY_CONFLICT"
            }
        );
        assert!(!root.join("adapter").exists());
        assert!(!root.join("QQ.app").exists());
    }
    source.revalidate(deadline).unwrap();
}

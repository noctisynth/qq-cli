use super::*;
use std::time::Duration;

#[test]
fn package_reads_are_bounded_and_bound_to_the_original_tree_and_file() {
    for variant in 0..7 {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("Original.app");
        let resources = root.join("Contents/Resources/app");
        std::fs::create_dir_all(&resources).unwrap();
        let package = resources.join("package.json");
        let bytes = if variant == 1 {
            vec![b' '; 1024 * 1024 + 1]
        } else {
            b"original package".to_vec()
        };
        std::fs::write(&package, &bytes).unwrap();
        std::fs::write(root.join("other"), b"original").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let inventory = Inventory::inspect(&root, deadline).unwrap();
        match variant {
            2 => std::fs::write(&package, b"modified package").unwrap(),
            3 => {
                std::fs::rename(&package, base.join("saved-package")).unwrap();
                std::fs::write(&package, &bytes).unwrap();
            }
            4 => std::fs::write(root.join("other"), b"modified").unwrap(),
            5 => {
                std::fs::rename(&resources, base.join("saved-resources")).unwrap();
                std::os::unix::fs::symlink(base.join("saved-resources"), &resources).unwrap();
            }
            _ => {}
        }
        let result = inventory.package_bytes(if variant == 6 {
            Instant::now()
        } else {
            deadline
        });
        if variant == 0 {
            assert_eq!(result.unwrap(), bytes);
            inventory.revalidate(deadline).unwrap();
        } else {
            assert!(result.is_err(), "variant {variant}");
        }
    }
}

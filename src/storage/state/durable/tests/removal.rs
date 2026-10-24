use super::*;
use std::os::unix::fs::symlink;

#[test]
fn relative_removal_syncs_only_the_expected_regular_file() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    std::fs::write(root.join("owned"), b"owned pointer").unwrap();
    std::fs::write(root.join("other"), b"unrelated pointer").unwrap();
    let directory = Directory::open(&root).unwrap();
    let held = File::open(root.join("owned")).unwrap();
    directory.remove_file("owned", &held).unwrap();
    assert!(!root.join("owned").exists());
    assert_eq!(
        std::fs::read(root.join("other")).unwrap(),
        b"unrelated pointer"
    );
}

#[test]
fn relative_removal_rejects_replacement_links_parent_change_and_traversal() {
    for case in ["replacement", "link", "hardlink", "parent", "traversal"] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("directory");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("owned"), b"original pointer").unwrap();
        std::fs::write(base.join("outside"), b"outside marker").unwrap();
        let directory = Directory::open(&root).unwrap();
        let held = File::open(root.join("owned")).unwrap();
        match case {
            "replacement" | "link" => {
                std::fs::rename(root.join("owned"), root.join("original")).unwrap();
                if case == "link" {
                    symlink(base.join("outside"), root.join("owned")).unwrap();
                } else {
                    std::fs::write(root.join("owned"), b"replacement pointer").unwrap();
                }
            }
            "hardlink" => std::fs::hard_link(root.join("owned"), root.join("alias")).unwrap(),
            "parent" => {
                std::fs::rename(&root, base.join("old-directory")).unwrap();
                std::fs::create_dir(&root).unwrap();
                std::fs::write(root.join("owned"), b"replacement pointer").unwrap();
            }
            "traversal" => (),
            _ => unreachable!(),
        }
        let name = if case == "traversal" {
            "../outside"
        } else {
            "owned"
        };
        assert!(directory.remove_file(name, &held).is_err());
        assert!(root.join("owned").exists());
        assert_eq!(
            std::fs::read(base.join("outside")).unwrap(),
            b"outside marker"
        );
    }
}

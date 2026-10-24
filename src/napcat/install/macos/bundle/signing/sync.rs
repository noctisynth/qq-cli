//! Flush signed files, replacement directory entries and private-parent changes.
use super::*;

impl SigningPlan {
    pub(super) fn synchronize(
        &self,
        inventory: &Inventory,
        parent: &File,
        deadline: Instant,
        guard: &impl Fn() -> Result<()>,
    ) -> Result<()> {
        self.sync_using(inventory, parent, deadline, guard, &|file, directory| {
            if directory {
                file.sync_all().map_err(|_| invalid())
            } else {
                rustix::fs::fcntl_fullfsync(file).map_err(|_| invalid())
            }
        })
    }

    fn sync_using(
        &self,
        inventory: &Inventory,
        parent: &File,
        deadline: Instant,
        guard: &impl Fn() -> Result<()>,
        sync: &impl Fn(&File, bool) -> Result<()>,
    ) -> Result<()> {
        if self.root != inventory.root {
            return Err(invalid());
        }
        let check = || {
            crate::core::deadline::check(deadline)?;
            guard()?;
            crate::core::deadline::check(deadline)?;
            inventory.verify_stamps(deadline)
        };
        check()?;
        let mut files = BTreeSet::new();
        for target in &self.targets {
            files.insert(target.executable.as_path());
            if let Some(seal) = &target.seal {
                files.insert(seal.as_path());
            }
        }
        let mut directories: Vec<_> = files
            .iter()
            .filter_map(|path| path.parent())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        for path in &files {
            check()?;
            if !matches!(
                inventory.entries.get(*path).map(|e| &e.content),
                Some(Content::File { .. })
            ) {
                return Err(invalid());
            }
            let file = inventory.signing_file(path)?;
            check()?;
            sync(&file, false)?;
            check()?;
        }
        for path in directories {
            check()?;
            if !matches!(
                inventory.entries.get(path).map(|e| &e.content),
                Some(Content::Directory)
            ) {
                return Err(invalid());
            }
            let file = inventory.signing_file(path)?;
            check()?;
            sync(&file, true)?;
            check()?;
        }
        // Anonymous entitlement files were created/unlinked in this held parent.
        // Its identity is rechecked by the live-copy guard before and after sync.
        check()?;
        sync(parent, true)?;
        check()?;
        // A final device flush follows directory syncs. All application nodes
        // were inventoried on the same device; keep this actual signed file FD.
        let file = inventory.signing_file(files.first().ok_or_else(invalid)?)?;
        check()?;
        sync(&file, false)?;
        check()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    fn fixture() -> (tempfile::TempDir, Inventory, SigningPlan, File) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("Synthetic.app");
        std::fs::create_dir_all(root.join("Contents/MacOS")).unwrap();
        std::fs::create_dir_all(root.join("Contents/_CodeSignature")).unwrap();
        let executable = PathBuf::from("Contents/MacOS/Main");
        let seal = PathBuf::from("Contents/_CodeSignature/CodeResources");
        std::fs::copy(std::env::current_exe().unwrap(), root.join(&executable)).unwrap();
        std::fs::write(root.join(&seal), b"synthetic seal bytes").unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let inventory = Inventory::inspect(&root, deadline).unwrap();
        let mut file = inventory.signing_file(&executable).unwrap();
        let size = file.metadata().unwrap().len();
        let payload = ResignPayload::inspect(&mut file, size, deadline).unwrap();
        // A structural synchronization fixture, not a valid signed bundle.
        let plan = SigningPlan {
            root,
            fingerprint: inventory.fingerprint.clone(),
            targets: vec![SigningTarget {
                path: PathBuf::new(),
                executable,
                seal: Some(seal),
                policy: Policy::Main,
                identifier: Some("org.qq-cli.synthetic".into()),
                payload,
            }],
        };
        let parent = File::open(base).unwrap();
        (temp, inventory, plan, parent)
    }

    #[test]
    fn files_precede_directories_and_final_device_barrier_and_failures_stop_immediately() {
        let (_temp, inventory, plan, parent) = fixture();
        let deadline = Instant::now() + Duration::from_secs(10);
        let operations = RefCell::new(Vec::new());
        plan.sync_using(
            &inventory,
            &parent,
            deadline,
            &|| Ok(()),
            &|file, directory| {
                operations
                    .borrow_mut()
                    .push((file.metadata().unwrap().ino(), directory));
                Ok(())
            },
        )
        .unwrap();
        let operations = operations.into_inner();
        assert_eq!(
            operations.iter().map(|(_, dir)| *dir).collect::<Vec<_>>(),
            [false, false, true, true, true, false]
        );
        assert_eq!(operations[4].0, parent.metadata().unwrap().ino());
        assert_eq!(operations[5], operations[0]);
        for fail_at in 0..operations.len() {
            let count = Cell::new(0);
            let error = plan
                .sync_using(&inventory, &parent, deadline, &|| Ok(()), &|_, _| {
                    let at = count.get();
                    count.set(at + 1);
                    if at == fail_at {
                        Err(AppError::new("SYNC_FAILED", "Synthetic sync failure", 3))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            assert_eq!(error.code, "SYNC_FAILED");
            assert_eq!(count.get(), fail_at + 1);
        }
    }

    #[test]
    fn lost_guard_changed_file_and_expired_budget_never_flush_a_replacement() {
        let (_temp, inventory, plan, parent) = fixture();
        let calls = Cell::new(0);
        let sync = |_: &File, _: bool| {
            calls.set(calls.get() + 1);
            Ok(())
        };
        assert!(
            plan.sync_using(&inventory, &parent, Instant::now(), &|| Ok(()), &sync)
                .is_err()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        assert!(
            plan.sync_using(&inventory, &parent, deadline, &|| Err(invalid()), &sync)
                .is_err()
        );
        let path = inventory.root.join("Contents/MacOS/Main");
        std::fs::remove_file(&path).unwrap();
        std::fs::write(path, b"replacement").unwrap();
        assert!(
            plan.sync_using(&inventory, &parent, deadline, &|| Ok(()), &sync)
                .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
}

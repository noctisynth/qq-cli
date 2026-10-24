//! Cooperative signing transaction. Process-survival isolation remains separate.
use super::*;
use std::io::{Seek, SeekFrom, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

impl Inventory {
    /// Consume a live hardened copy and execute a fixed inside-out signing plan.
    /// This cooperative kernel must run under a production lifetime supervisor;
    /// it does not claim recovery after abrupt termination of this process.
    pub fn sign_copy(
        self,
        plan: &SigningPlan,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        self.sign_with_checks(plan, deadline, guard, false)
    }

    /// Supervise codesign independently and isolate blocking Policy checks.
    /// The host must register both private worker entries. Filesystem operations
    /// and persistent deployment recovery remain the caller's responsibility.
    pub fn sign_copy_supervised(
        self,
        plan: &SigningPlan,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<Self> {
        self.sign_with_checks(plan, deadline, guard, true)
    }

    fn sign_with_checks(
        mut self,
        plan: &SigningPlan,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
        isolated: bool,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let owner = self.owner.take().ok_or_else(invalid)?;
        let check = || {
            crate::core::deadline::check(deadline)?;
            owner.verify()?;
            guard()?;
            owner.verify()
        };
        check()?;
        plan.revalidate(&self, deadline)?;
        for (path, entry) in &self.entries {
            let expected = match entry.content {
                Content::Directory => 0o700,
                Content::File { .. } => 0o600 | (entry.mode & 0o100),
                Content::Link(_) => entry.mode,
            };
            if entry.mode != expected
                || entry.acl != crate::native::macos_acl::absent_digest()
                || self.stamps[path].uid != rustix::process::geteuid().as_raw()
            {
                return Err(invalid());
            }
        }
        // Establish all seal/layout constraints before the first signing effect.
        plan.verify_transition(&self, &self, deadline)?;
        let mut current = Self::inspect(&self.root, deadline)?;
        for target in &plan.targets {
            check()?;
            let input = entitlements(&owner.file, target.policy)?;
            check()?;
            current.verify_stamps(deadline)?;
            if isolated {
                let held = current.signing_file(&target.path)?;
                supervisor::sign(
                    &self.root.join(&target.path),
                    held,
                    input,
                    target.policy,
                    deadline,
                    &check,
                )?;
            } else {
                let child = Command::new("/usr/bin/codesign")
                    .args([
                        "--force",
                        "--sign",
                        "-",
                        "--timestamp=none",
                        "--options",
                        "runtime",
                        "--generate-entitlement-der",
                        "--entitlements",
                        "/dev/fd/0",
                    ])
                    .arg(self.root.join(&target.path))
                    .env_clear()
                    .current_dir("/")
                    .stdin(Stdio::from(input))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|_| invalid())?;
                wait(child, deadline, &check)?;
            }
            check()?;
            let next = Self::inspect_using(&self.root, deadline, Some(&current))?;
            target.verify_step(&current, &next, deadline)?;
            current = next;
            verify(
                target.policy,
                &self.root.join(&target.path),
                deadline,
                &check,
                isolated,
            )?;
            check()?;
            current.verify_stamps(deadline)?;
        }
        for target in &plan.targets {
            check()?;
            verify(
                target.policy,
                &self.root.join(&target.path),
                deadline,
                &check,
                isolated,
            )?;
        }
        plan.verify_transition(&self, &current, deadline)?;
        plan.synchronize(&current, &owner.file, deadline, &check)?;
        check()?;
        current.verify_stamps(deadline)?;
        current.owner = Some(owner);
        current.signed = true;
        Ok(current)
    }
}

fn verify(
    policy: Policy,
    path: &Path,
    deadline: Instant,
    guard: &impl Fn() -> Result<()>,
    isolated: bool,
) -> Result<()> {
    if isolated {
        policy.verify_isolated(path, deadline, guard)
    } else {
        policy.verify(path, deadline)
    }
}

fn entitlements(parent: &File, policy: Policy) -> Result<File> {
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| invalid())?;
    let name = format!(".qq-cli-entitlements-{:032x}", u128::from_le_bytes(random));
    let mut file = File::from(
        rustix::fs::openat(
            parent,
            name.as_str(),
            rustix::fs::OFlags::RDWR
                | rustix::fs::OFlags::CREATE
                | rustix::fs::OFlags::EXCL
                | rustix::fs::OFlags::NOFOLLOW
                | rustix::fs::OFlags::CLOEXEC,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .map_err(|_| invalid())?,
    );
    // Unlink before storing bytes. codesign needs a seekable file, not a pipe;
    // /dev/fd/0 addresses the inherited descriptor, never this temporary name.
    rustix::fs::unlinkat(parent, name.as_str(), rustix::fs::AtFlags::empty())
        .map_err(|_| invalid())?;
    let meta = file.metadata().map_err(|_| invalid())?;
    if !meta.is_file()
        || meta.nlink() != 0
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o777 != 0o600
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(file.as_fd())?;
    file.write_all(policy.entitlements().as_bytes())
        .map_err(|_| invalid())?;
    file.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
    Ok(file)
}

struct Owned(Option<Child>);
impl Owned {
    fn abort(&mut self, error: AppError, may_signal: bool) -> AppError {
        let Some(mut child) = self.0.take() else {
            return error;
        };
        match child.try_wait() {
            Ok(Some(_)) => return error,
            Ok(None) if may_signal => {
                let _ = child.kill();
            }
            _ => {} // Never signal after an uncertain wait result.
        }
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return error;
            }
            if Instant::now() >= end {
                return AppError::new(
                    "SIGNING_CLEANUP_UNKNOWN",
                    "Signing process exit was not confirmed; retain the partial copy and do not retry.",
                    3,
                );
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.abort(invalid(), true);
    }
}
pub(super) fn wait(child: Child, deadline: Instant, guard: &impl Fn() -> Result<()>) -> Result<()> {
    let mut child = Owned(Some(child));
    loop {
        if let Err(error) = crate::core::deadline::check(deadline).and_then(|_| guard()) {
            return Err(child.abort(error, true));
        }
        match child.0.as_mut().ok_or_else(invalid)?.try_wait() {
            Ok(Some(status)) => {
                child.0.take();
                crate::core::deadline::check(deadline)?;
                guard()?;
                return if status.success() {
                    Ok(())
                } else {
                    Err(invalid())
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => return Err(child.abort(invalid(), false)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn child(mode: &str) -> Child {
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "napcat::install::macos_bundle::signing::execute::tests::fixture",
                "--nocapture",
            ])
            .env("QQ_CLI_SIGNING_FIXTURE", mode)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    }
    #[test]
    fn fixture() {
        let Ok(mode) = std::env::var("QQ_CLI_SIGNING_FIXTURE") else {
            return;
        };
        if mode == "success" {
            return;
        }
        if mode == "failure" {
            std::process::exit(9);
        }
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    #[test]
    fn exit_status_deadline_and_guard_are_independent_requirements() {
        let end = || Instant::now() + Duration::from_secs(5);
        wait(child("success"), end(), &|| Ok(())).unwrap();
        assert!(wait(child("failure"), end(), &|| Ok(())).is_err());
        for expired in [false, true] {
            let worker = child("hang");
            let observed = crate::native::process::Pinned::observe(worker.id(), end()).unwrap();
            let start = Instant::now();
            let result = if expired {
                wait(worker, Instant::now(), &|| Ok(()))
            } else {
                wait(worker, end(), &|| {
                    Err(AppError::new("GUARD_LOST", "Synthetic guard loss", 3))
                })
            };
            assert_eq!(
                result.unwrap_err().code,
                if expired { "DEADLINE" } else { "GUARD_LOST" }
            );
            assert!(start.elapsed() < Duration::from_secs(3));
            assert!(observed.verify(end()).is_err());
        }
    }
}

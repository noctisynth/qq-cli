//! Separate owner of the codesign Child; cancellation never kills its supervisor.
use super::*;
use std::io::{Seek, SeekFrom};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

const FLAG: &str = "--internal-macos-signer-v1";
const SIGNED: i32 = 109;
const FAILED: i32 = 110;
const EXPIRED: i32 = 111;
const UNKNOWN: i32 = 112;

pub(super) fn sign(
    path: &Path,
    target: File,
    input: File,
    policy: Policy,
    deadline: Instant,
    guard: &impl Fn() -> Result<()>,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let program = crate::native::process::current_executable(deadline)?;
    let budget = deadline
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(1_800_000);
    if budget == 0 {
        return Err(expired());
    }
    let role = match policy {
        Policy::Main => "main",
        Policy::Electron => "electron",
        Policy::PluginHost => "plugin-host",
        Policy::Native => "native",
    };
    let child = Command::new(program)
        .arg(FLAG)
        .arg(path)
        .arg(role)
        .arg(budget.to_string())
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(input))
        .stderr(Stdio::from(target))
        .spawn()
        .map_err(|_| invalid())?;
    await_supervisor(child, deadline, guard)
}

struct Supervisor(Option<Child>);
impl Supervisor {
    fn cancel(&mut self, cause: AppError) -> AppError {
        let Some(mut child) = self.0.take() else {
            return cause;
        };
        // EOF asks the still-live supervisor to kill AND reap its own child.
        // Killing the supervisor here could strand a writer, so never do that.
        child.stdin.take();
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if matches!(status.code(), Some(SIGNED | FAILED | EXPIRED)) {
                        cause
                    } else {
                        unknown()
                    };
                }
                Err(_) => return unknown(),
                Ok(None) if Instant::now() >= end => return unknown(),
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.cancel(invalid());
    }
}
fn await_supervisor(
    child: Child,
    deadline: Instant,
    guard: &impl Fn() -> Result<()>,
) -> Result<()> {
    let mut child = Supervisor(Some(child));
    loop {
        if let Err(error) = crate::core::deadline::check(deadline).and_then(|_| guard()) {
            return Err(child.cancel(error));
        }
        match child.0.as_mut().ok_or_else(invalid)?.try_wait() {
            Ok(Some(status)) => {
                child.0.take();
                match status.code() {
                    Some(SIGNED) => {
                        crate::core::deadline::check(deadline)?;
                        guard()?;
                        return Ok(());
                    }
                    Some(FAILED) => return Err(invalid()),
                    Some(EXPIRED) => return Err(expired()),
                    _ => return Err(unknown()),
                }
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(_) => return Err(child.cancel(unknown())),
        }
    }
}

/// stdin is parent liveness, stdout is an unlinked entitlement file, stderr is
/// a held target descriptor. This mode never writes to the inherited streams.
pub fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return None;
    }
    let result = (|| {
        let (Some(path), Some(role), Some(budget), None) =
            (args.next(), args.next(), args.next(), args.next())
        else {
            return Err(invalid());
        };
        let policy = match role.to_str() {
            Some("main") => Policy::Main,
            Some("electron") => Policy::Electron,
            Some("plugin-host") => Policy::PluginHost,
            Some("native") => Policy::Native,
            _ => return Err(invalid()),
        };
        let ms = budget
            .to_str()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|ms| (1..=1_800_000).contains(ms))
            .ok_or_else(invalid)?;
        let deadline = Instant::now() + Duration::from_millis(ms);
        let alive = watch_parent()?;
        let check = || {
            crate::core::deadline::check(deadline)?;
            if alive.load(Ordering::Acquire) {
                Ok(())
            } else {
                Err(invalid())
            }
        };
        check()?;
        let mut input = File::from(rustix::io::dup(std::io::stdout()).map_err(|_| invalid())?);
        let target = File::from(rustix::io::dup(std::io::stderr()).map_err(|_| invalid())?);
        let meta = input.metadata().map_err(|_| invalid())?;
        if !meta.is_file()
            || meta.nlink() != 0
            || meta.mode() & 0o777 != 0o600
            || meta.uid() != rustix::process::geteuid().as_raw()
            || meta.len() > 4096
        {
            return Err(invalid());
        }
        crate::native::macos_acl::require_absent(input.as_fd())?;
        input.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut input)
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if bytes != policy.entitlements().as_bytes() {
            return Err(invalid());
        }
        input.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
        let path = Path::new(&path);
        reject_links(path)?;
        if !path.is_absolute() || path.canonicalize().map_err(|_| invalid())? != path {
            return Err(invalid());
        }
        let held = target.metadata().map_err(|_| invalid())?;
        let current = std::fs::symlink_metadata(path).map_err(|_| invalid())?;
        if Stamp::from(&held) != Stamp::from(&current)
            || held.uid() != rustix::process::geteuid().as_raw()
            || (!held.is_file() && !held.is_dir())
            || held.mode() & 0o077 != 0
        {
            return Err(invalid());
        }
        crate::native::macos_acl::require_absent(target.as_fd())?;
        check()?;
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
            .arg(path)
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::from(input))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| invalid())?;
        execute::wait(child, deadline, &check)
    })();
    Some(match result {
        Ok(()) => SIGNED,
        Err(error) if error.code == "SIGNING_CLEANUP_UNKNOWN" => UNKNOWN,
        Err(error) if error.code == "DEADLINE" => EXPIRED,
        Err(_) => FAILED,
    } as u8)
}

fn watch_parent() -> Result<Arc<AtomicBool>> {
    let alive = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&alive);
    std::thread::Builder::new()
        .name("signer-parent".into())
        .spawn(move || {
            let _ = std::io::stdin().read(&mut [0_u8; 1]);
            flag.store(false, Ordering::Release);
        })
        .map_err(|_| invalid())?;
    Ok(alive)
}
fn expired() -> AppError {
    AppError::new("DEADLINE", "Operation deadline exceeded.", 3)
}
fn unknown() -> AppError {
    AppError::new(
        "SIGNING_CLEANUP_UNKNOWN",
        "Signing process exit was not confirmed; retain the partial copy and do not retry.",
        3,
    )
}

#[cfg(test)]
mod tests;

//! A disposable self-worker owns the blocking Security Framework call.
use super::{Origin, invalid};
use crate::core::error::{AppError, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const FLAG: &str = "--internal-macos-signature-v1";
const DEVELOPER: u8 = 91;
const STORE: u8 = 92;
const FAILURE: u8 = 93;
const TIMEOUT: u8 = 94;

/// Only the currently running CLI is used as worker. No user-selected checker,
/// inherited loader environment, shell, output parsing or credentials.
pub fn verify_isolated(root: &Path, deadline: Instant) -> Result<Origin> {
    crate::core::deadline::check(deadline)?;
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(300));
    let milliseconds = remaining.as_millis();
    if milliseconds == 0 {
        return Err(timeout());
    }
    let program = std::env::current_exe().map_err(|_| invalid())?;
    crate::storage::state::reject_links(&program)?;
    let child = Command::new(program)
        .args([
            std::ffi::OsStr::new(FLAG),
            root.as_os_str(),
            std::ffi::OsStr::new(&milliseconds.to_string()),
        ])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| invalid())?;
    wait(child, deadline.min(Instant::now() + remaining))
}

struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        // Closing the liveness pipe also tells the worker to exit if this parent
        // unwinds. Only this unreaped direct child may be signalled here.
        self.0.stdin.take();
        if matches!(self.0.try_wait(), Ok(None)) {
            let _ = self.0.kill();
        }
    }
}

fn wait(child: Child, deadline: Instant) -> Result<Origin> {
    let mut child = Owned(child);
    loop {
        match child.0.try_wait() {
            Ok(Some(status)) => {
                crate::core::deadline::check(deadline)?;
                return match status.code() {
                    Some(code) if code == i32::from(DEVELOPER) => Ok(Origin::TencentDeveloperId),
                    Some(code) if code == i32::from(STORE) => Ok(Origin::AppleAppStore),
                    Some(code) if code == i32::from(TIMEOUT) => Err(timeout()),
                    _ => Err(invalid()),
                };
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(5)),
                );
            }
            result => {
                let failed = result.is_err();
                child.0.stdin.take();
                if !failed {
                    let _ = child.0.kill();
                }
                // Reap with a separate bounded cleanup allowance. A kernel which
                // cannot reap is reported as unknown, never as a verified exit.
                let cleanup = Instant::now() + Duration::from_secs(2);
                loop {
                    if matches!(child.0.try_wait(), Ok(Some(_))) {
                        return Err(if failed { invalid() } else { timeout() });
                    }
                    if Instant::now() >= cleanup {
                        return Err(AppError::new(
                            "SIGNATURE_WORKER_CLEANUP_UNKNOWN",
                            "Signature checker did not confirm exit within the cleanup allowance; no signature result was accepted.",
                            3,
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }
}

/// Called before public argument parsing. This private, read-only worker has no
/// state/credential/QQ lifecycle access and never emits public CLI output.
pub fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return None;
    }
    let (Some(root), Some(budget), None) = (args.next(), args.next(), args.next()) else {
        return Some(FAILURE);
    };
    let Some(milliseconds) = budget
        .to_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| (1..=300_000).contains(v))
    else {
        return Some(FAILURE);
    };
    let duration = Duration::from_millis(milliseconds);
    let deadline = Instant::now() + duration;
    if watchdogs(duration).is_err() {
        return Some(FAILURE);
    }
    Some(match super::verify_original(Path::new(&root), deadline) {
        Ok(Origin::TencentDeveloperId) => DEVELOPER,
        Ok(Origin::AppleAppStore) => STORE,
        Err(error) if error.code == "DEADLINE" => TIMEOUT,
        Err(_) => FAILURE,
    })
}

fn watchdogs(duration: Duration) -> std::result::Result<(), ()> {
    if std::thread::Builder::new()
        .name("signature-parent".into())
        .spawn(|| {
            let _ = std::io::stdin().read(&mut [0_u8; 1]);
            std::process::exit(i32::from(FAILURE));
        })
        .is_err()
    {
        return Err(());
    }
    if std::thread::Builder::new()
        .name("signature-deadline".into())
        .spawn(move || {
            std::thread::sleep(duration);
            std::process::exit(i32::from(TIMEOUT));
        })
        .is_err()
    {
        return Err(());
    }
    Ok(())
}

fn timeout() -> AppError {
    AppError::new("DEADLINE", "Operation deadline exceeded.", 3)
}

#[cfg(test)]
#[path = "worker/tests.rs"]
mod tests;

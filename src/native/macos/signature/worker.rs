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
pub(crate) const FAILURE: u8 = 93;
pub(crate) const TIMEOUT: u8 = 94;

/// Only the currently running CLI is used as worker. No user-selected checker,
/// inherited loader environment, shell, output parsing or credentials.
pub fn verify_isolated(root: &Path, deadline: Instant) -> Result<Origin> {
    origin(check_worker(
        FLAG,
        &[root.as_os_str()],
        deadline,
        Duration::from_secs(300),
        &|| Ok(()),
    )?)
}

pub(crate) fn check_worker(
    flag: &str,
    parameters: &[&std::ffi::OsStr],
    deadline: Instant,
    maximum: Duration,
    guard: &impl Fn() -> Result<()>,
) -> Result<u8> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let remaining = worker_budget(deadline, maximum)?;
    let milliseconds = remaining.as_millis();
    let program = crate::native::process::current_executable(deadline)?;
    crate::core::deadline::check(deadline)?;
    guard()?;
    let child = Command::new(program)
        .arg(flag)
        .args(parameters)
        .arg(milliseconds.to_string())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| invalid())?;
    wait_code(child, deadline.min(Instant::now() + remaining), guard)
}

fn worker_budget(deadline: Instant, maximum: Duration) -> Result<Duration> {
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .min(maximum);
    if remaining.as_millis() == 0 {
        return Err(timeout());
    }
    Ok(remaining)
}

struct Owned(Option<Child>);
impl Drop for Owned {
    fn drop(&mut self) {
        // Closing the liveness pipe also tells the worker to exit if this parent
        // unwinds. Only this unreaped direct child may be signalled here.
        if let Some(child) = self.0.as_mut() {
            child.stdin.take();
            if matches!(child.try_wait(), Ok(None)) {
                let _ = child.kill();
            }
        }
    }
}

fn origin(code: u8) -> Result<Origin> {
    match code {
        DEVELOPER => Ok(Origin::TencentDeveloperId),
        STORE => Ok(Origin::AppleAppStore),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
fn wait(child: Child, deadline: Instant) -> Result<Origin> {
    origin(wait_code(child, deadline, &|| Ok(()))?)
}

fn wait_code(child: Child, deadline: Instant, guard: &impl Fn() -> Result<()>) -> Result<u8> {
    let mut child = Owned(Some(child));
    loop {
        let checked = crate::core::deadline::check(deadline).and_then(|_| guard());
        if let Err(error) = checked {
            return Err(abort(&mut child, error, true));
        }
        match child.0.as_mut().ok_or_else(invalid)?.try_wait() {
            Ok(Some(status)) => {
                child.0.take();
                crate::core::deadline::check(deadline)?;
                guard()?;
                return match status.code() {
                    Some(code) if code == i32::from(TIMEOUT) => Err(timeout()),
                    Some(code) => u8::try_from(code).map_err(|_| invalid()),
                    _ => Err(invalid()),
                };
            }
            Ok(None) => {
                std::thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(5)),
                );
            }
            Err(_) => return Err(abort(&mut child, invalid(), false)),
        }
    }
}

fn abort(owned: &mut Owned, error: AppError, may_signal: bool) -> AppError {
    let Some(mut child) = owned.0.take() else {
        return error;
    };
    child.stdin.take();
    match child.try_wait() {
        Ok(Some(_)) => return error,
        Ok(None) if may_signal => {
            let _ = child.kill();
        }
        _ => {}
    }
    let cleanup = Instant::now() + Duration::from_secs(2);
    loop {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return error;
        }
        if Instant::now() >= cleanup {
            return AppError::new(
                "SIGNATURE_WORKER_CLEANUP_UNKNOWN",
                "Signature checker did not confirm exit within the cleanup allowance; no signature result was accepted.",
                3,
            );
        }
        std::thread::sleep(Duration::from_millis(5));
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

pub(crate) fn watchdogs(duration: Duration) -> std::result::Result<(), ()> {
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

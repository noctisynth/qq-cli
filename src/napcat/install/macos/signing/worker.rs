//! Read-only exact-policy checking in the current CLI's disposable worker.
use super::*;
use crate::native::macos_signature::{FAILURE, TIMEOUT, check_worker, watchdogs};
use std::ffi::OsStr;
use std::time::Duration;

const FLAG: &str = "--internal-macos-policy-v1";
const VERIFIED: u8 = 95;

impl Policy {
    /// The hosting executable must dispatch worker_entry before public parsing.
    /// The caller still owns inventory/path authority and effect-boundary checks.
    pub fn verify_isolated(
        self,
        path: &Path,
        deadline: Instant,
        guard: impl Fn() -> Result<()>,
    ) -> Result<()> {
        let role = match self {
            Self::Main => "main",
            Self::Electron => "electron",
            Self::PluginHost => "plugin-host",
            Self::Native => "native",
        };
        if check_worker(
            FLAG,
            &[path.as_os_str(), OsStr::new(role)],
            deadline,
            Duration::from_secs(300),
            &guard,
        )? != VERIFIED
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// No signing, credentials, QQ execution or state mutation in this private mode.
pub fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(FLAG)) {
        return None;
    }
    let (Some(path), Some(role), Some(budget), None) =
        (args.next(), args.next(), args.next(), args.next())
    else {
        return Some(FAILURE);
    };
    let policy = match role.to_str() {
        Some("main") => Policy::Main,
        Some("electron") => Policy::Electron,
        Some("plugin-host") => Policy::PluginHost,
        Some("native") => Policy::Native,
        _ => return Some(FAILURE),
    };
    let Some(ms) = budget
        .to_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| (1..=300_000).contains(v))
    else {
        return Some(FAILURE);
    };
    let duration = Duration::from_millis(ms);
    let deadline = Instant::now() + duration;
    if watchdogs(duration).is_err() {
        return Some(FAILURE);
    }
    Some(match policy.verify(Path::new(&path), deadline) {
        Ok(()) => VERIFIED,
        Err(error) if error.code == "DEADLINE" => TIMEOUT,
        Err(_) => FAILURE,
    })
}

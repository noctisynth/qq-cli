//! Production release dispatch: inherited original devices, fixed unmount and one eject.
use super::supervision::{
    EXPIRED, FAILED, FINISHED, Parent, Supervisor, UNKNOWN, expired, invalid,
};
use super::*;
use crate::storage::{references::References, state::Store};
use std::{
    ffi::OsStr,
    io::Write,
    process::{Command, Stdio},
    time::Duration,
};
const FLAG: &str = "--internal-macos-release-v1";

pub fn release(
    store: &Store,
    references: &References,
    mount_request: &str,
    request: &str,
    image: Image,
    deadline: Instant,
) -> Result<journal::release::Status> {
    journal::release::release(
        store,
        references,
        mount_request,
        request,
        image,
        deadline,
        |pins, guard| dispatch(pins, deadline, guard),
    )
}
fn dispatch(
    pins: mounted::Devices,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    let (transfer, whole, volume) = pins.into_transfer(deadline, guard)?;
    let metadata = serde_json::to_string(&transfer).map_err(|_| invalid())?;
    if metadata.len() > 4096 {
        return Err(invalid());
    }
    let program = std::env::current_exe().map_err(|_| invalid())?;
    reject_links(&program)?;
    let budget = deadline
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(300_000);
    if budget == 0 {
        return Err(expired());
    }
    guard()?;
    let child = Command::new(program)
        .arg(FLAG)
        .arg(metadata)
        .arg(budget.to_string())
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(whole))
        .stderr(volume.map(Stdio::from).unwrap_or_else(Stdio::null))
        .spawn()
        .map_err(|_| invalid())?;
    let mut supervisor = Supervisor(Some(child));
    if let Err(error) = guard().and_then(|_| crate::core::deadline::check(deadline)) {
        return Err(supervisor.cancel(error));
    }
    if supervisor
        .0
        .as_mut()
        .and_then(|child| child.stdin.as_mut())
        .ok_or_else(invalid)?
        .write_all(&[1])
        .is_err()
    {
        return Err(supervisor.cancel(invalid()));
    }
    supervisor.wait(deadline, guard)
}

/// stdout/stderr carry device FDs, not output. Replace them before dropping any
/// volume pin; otherwise the inherited original stderr alone can keep it busy.
pub fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(FLAG)) {
        return None;
    }
    let result = (|| {
        let (Some(metadata), Some(budget), None) = (args.next(), args.next(), args.next()) else {
            return Err(invalid());
        };
        let metadata = metadata
            .to_str()
            .filter(|v| v.len() <= 4096)
            .ok_or_else(invalid)?;
        let transfer: mounted::Transfer = serde_json::from_str(metadata).map_err(|_| invalid())?;
        let ms = budget
            .to_str()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|v| (1..=300_000).contains(v))
            .ok_or_else(invalid)?;
        let deadline = Instant::now() + Duration::from_millis(ms);
        let parent = Parent::prepare(deadline)?;
        let whole = File::from(rustix::io::dup(std::io::stdout()).map_err(|_| invalid())?);
        let volume = if transfer.has_volume() {
            Some(File::from(
                rustix::io::dup(std::io::stderr()).map_err(|_| invalid())?,
            ))
        } else {
            None
        };
        for file in std::iter::once(&whole).chain(volume.iter()) {
            rustix::io::fcntl_setfd(file, rustix::io::FdFlags::CLOEXEC).map_err(|_| invalid())?;
        }
        let null = File::options()
            .read(true)
            .write(true)
            .open("/dev/null")
            .map_err(|_| invalid())?;
        rustix::stdio::dup2_stdout(&null).map_err(|_| invalid())?;
        rustix::stdio::dup2_stderr(&null).map_err(|_| invalid())?;
        let guard = || parent.check(deadline);
        let pins = mounted::Devices::from_transfer(transfer, whole, volume, deadline, &guard)?;
        guard()?;
        mounted::command::run(
            "/usr/sbin/diskutil",
            &[OsStr::new("unmount"), OsStr::new(pins.volume_name())],
            None,
            deadline,
            &guard,
        )?;
        pins.eject_once(deadline, &guard)
    })();
    Some(match result {
        Ok(()) => FINISHED,
        Err(e) if e.code == "MOUNT_QUERY_EXIT_UNKNOWN" => UNKNOWN,
        Err(e) if e.code == "DEADLINE" => EXPIRED,
        Err(_) => FAILED,
    } as u8)
}

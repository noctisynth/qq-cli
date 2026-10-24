//! Fixed read-only attach under a durable intent and an independent supervisor.
use super::supervision::{
    EXPIRED, FAILED, FINISHED, Parent, Supervisor, UNKNOWN, expired, invalid,
};
use super::*;
use crate::storage::{references::References, state::Store};
use std::{
    io::Write,
    process::{Command, Stdio},
};
mod worker;
pub(in crate::napcat::install::macos) use worker::worker_entry;
const FLAG: &str = "--internal-macos-mount-v1";

/// The current executable must register macos::worker_entry before parsing public CLI args.
/// Success still comes from journal verification, never from command exit alone.
pub fn attach(
    store: &Store,
    references: &References,
    request: &str,
    image: Image,
    deadline: Instant,
) -> Result<journal::Status> {
    journal::attach(
        store,
        references,
        request,
        image,
        deadline,
        |image, mount, guard| dispatch(image, mount, deadline, guard),
    )
}
fn dispatch(
    image: &Path,
    mount: &Path,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let input = open(image)?;
    acquire::private_file(&input)?;
    let root = acquire::directory(mount)?;
    worker::verify_inputs(image, mount, &input, &root)?;
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
        .arg(image)
        .arg(mount)
        .arg(budget.to_string())
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(input))
        .stderr(Stdio::from(root))
        .spawn()
        .map_err(|_| invalid())?;
    let mut supervisor = Supervisor(Some(child));
    // No effect is allowed merely by starting a worker. Recheck the durable
    // intent after spawn, then give its one-byte dispatch permission.
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

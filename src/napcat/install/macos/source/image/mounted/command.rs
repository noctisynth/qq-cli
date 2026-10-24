//! Bounded read-only system queries; child exit never proves mount cleanup.
use super::*;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const LIMIT: usize = 4 * 1024 * 1024;

pub(super) fn info(deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    let plist = run(
        "/usr/bin/hdiutil",
        &[OsStr::new("info"), OsStr::new("-plist")],
        None,
        deadline,
        guard,
    )?;
    json(&plist, deadline, guard)
}

pub(super) fn media(deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    let plist = run(
        "/usr/sbin/ioreg",
        &["-a", "-r", "-c", "IOMedia"].map(OsStr::new),
        None,
        deadline,
        guard,
    )?;
    json(&plist, deadline, guard)
}

fn json(plist: &[u8], deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<Vec<u8>> {
    run(
        "/usr/bin/plutil",
        &["-convert", "json", "-o", "-", "--", "-"].map(OsStr::new),
        Some(plist),
        deadline,
        guard,
    )
}

pub(in crate::napcat::install::macos::source::image) fn run(
    program: &str,
    args: &[&OsStr],
    input: Option<&[u8]>,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    crate::core::deadline::check(deadline)?;
    guard()?;
    let stdin = if let Some(bytes) = input {
        if bytes.len() > LIMIT {
            return Err(invalid());
        }
        let mut file = tempfile::tempfile().map_err(|_| invalid())?;
        file.write_all(bytes).map_err(|_| invalid())?;
        file.seek(SeekFrom::Start(0)).map_err(|_| invalid())?;
        Stdio::from(file)
    } else {
        Stdio::null()
    };
    guard()?;
    crate::core::deadline::check(deadline)?;
    let child = Command::new(program)
        .args(args)
        .env_clear()
        .current_dir("/")
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| invalid())?;
    let mut owned = Owned(Some(child));
    let result = capture(&mut owned, deadline, guard);
    match result {
        Ok(bytes) => {
            owned.0.take();
            Ok(bytes)
        }
        Err(error) => Err(owned.cancel(error)),
    }
}

fn capture(
    owned: &mut Owned,
    deadline: Instant,
    guard: &dyn Fn() -> Result<()>,
) -> Result<Vec<u8>> {
    let child = owned.0.as_mut().ok_or_else(invalid)?;
    let mut output = child.stdout.take().ok_or_else(invalid)?;
    let flags = rustix::fs::fcntl_getfl(&output).map_err(|_| invalid())?;
    rustix::fs::fcntl_setfl(&output, flags | rustix::fs::OFlags::NONBLOCK)
        .map_err(|_| invalid())?;
    let mut bytes = Vec::new();
    let mut status = None;
    let mut eof = false;
    loop {
        crate::core::deadline::check(deadline)?;
        guard()?;
        if !eof {
            let mut buffer = [0; 16384];
            match output.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if count > LIMIT - bytes.len() {
                        return Err(invalid());
                    }
                    bytes.extend_from_slice(&buffer[..count]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(invalid()),
            }
        }
        if status.is_none() {
            status = child.try_wait().map_err(|_| unknown())?;
        }
        if let Some(status) = status {
            if !status.success() {
                return Err(invalid());
            }
            if eof {
                guard()?;
                crate::core::deadline::check(deadline)?;
                return Ok(bytes);
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

struct Owned(Option<Child>);
impl Owned {
    fn cancel(&mut self, error: AppError) -> AppError {
        let Some(mut child) = self.0.take() else {
            return error;
        };
        match child.try_wait() {
            Ok(Some(_)) => return error,
            Ok(None) => {
                let _ = child.kill();
            }
            Err(_) => return unknown(),
        }
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => return error,
                Err(_) => return unknown(),
                Ok(None) if Instant::now() >= end => return unknown(),
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.cancel(invalid());
    }
}
fn unknown() -> AppError {
    AppError::new(
        "MOUNT_QUERY_EXIT_UNKNOWN",
        "Read-only mount query exit was not confirmed; no mount ownership or cleanup was inferred.",
        3,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_exit_deadline_and_guard_are_independent() {
        let call = |script: &str, duration, guard: &dyn Fn() -> Result<()>| {
            run(
                "/bin/sh",
                &[OsStr::new("-c"), OsStr::new(script)],
                None,
                Instant::now() + duration,
                guard,
            )
        };
        let budget = Duration::from_secs(5);
        assert_eq!(
            call("printf synthetic", budget, &|| Ok(())).unwrap(),
            b"synthetic"
        );
        assert!(call("printf synthetic; exit 1", budget, &|| Ok(())).is_err());
        assert!(call("exec /usr/bin/yes", budget, &|| Ok(())).is_err());
        assert!(call("exec /bin/sleep 10", Duration::from_millis(50), &|| Ok(())).is_err());
        let checks = std::cell::Cell::new(0);
        assert!(
            call("exec /bin/sleep 10", budget, &|| {
                checks.set(checks.get() + 1);
                if checks.get() > 3 {
                    Err(invalid())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
    }
}

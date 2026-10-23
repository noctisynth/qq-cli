//! Persistent self-worker for blocking copies; only passed descriptors are writable.
use crate::core::error::{AppError, Result};
use std::io::{Read, Write};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
mod wire;

const FLAG: &str = "--internal-macos-copy-v1";
const SUCCESS: u8 = 101;
const FAILURE: u8 = 102;
const TIMEOUT: u8 = 103;

/// Host binaries embedding this capability must register worker_entry before
/// their normal argument parser. No alternate worker path is accepted.
pub struct Worker {
    child: Child,
    stream: UnixStream,
    deadline: Instant,
    terminal: bool,
    cleanup_attempted: bool,
}
impl Worker {
    pub fn start(deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        let duration = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_secs(1800));
        if duration.as_millis() == 0 {
            return Err(timeout());
        }
        let program = std::env::current_exe().map_err(|_| failed())?;
        crate::storage::state::reject_links(&program)?;
        let mut command = Command::new(program);
        command
            .arg(FLAG)
            .arg(duration.as_millis().to_string())
            .env_clear();
        Self::spawn(command, deadline.min(Instant::now() + duration))
    }
    fn spawn(mut command: Command, deadline: Instant) -> Result<Self> {
        let (stream, child_socket) = UnixStream::pair().map_err(|_| failed())?;
        let fd: OwnedFd = child_socket.into();
        let child = command
            .stdin(Stdio::from(fd))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| failed())?;
        let mut worker = Self {
            child,
            stream,
            deadline,
            terminal: false,
            cleanup_attempted: false,
        };
        if worker.stream.set_nonblocking(true).is_err() {
            return Err(worker.abort(failed()));
        }
        Ok(worker)
    }
    pub fn copy(
        &mut self,
        source: BorrowedFd<'_>,
        destination: BorrowedFd<'_>,
        data: bool,
        guard: impl Fn() -> Result<()>,
    ) -> Result<()> {
        if self.terminal || self.cleanup_attempted {
            return Err(failed());
        }
        let operation = (|| {
            self.check(&guard)?;
            wire::validate(source, destination, data)?;
            wire::send(
                &self.stream,
                if data { 2 } else { 1 },
                &[source, destination],
            )?;
            loop {
                self.check(&guard)?;
                let mut reply = [0_u8];
                match self.stream.read(&mut reply) {
                    Ok(1) if reply[0] == SUCCESS => {
                        self.check(&guard)?;
                        return Ok(());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => (),
                    _ => return Err(failed()),
                }
                if self.child.try_wait().map_err(|_| failed())?.is_some() {
                    return Err(failed());
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })();
        operation.map_err(|error| self.abort(error))
    }
    pub fn finish(mut self, guard: impl Fn() -> Result<()>) -> Result<()> {
        let operation = (|| {
            if self.terminal || self.cleanup_attempted {
                return Err(failed());
            }
            self.check(&guard)?;
            wire::send(&self.stream, 0, &[])?;
            loop {
                self.check(&guard)?;
                if let Some(status) = self.child.try_wait().map_err(|_| failed())? {
                    self.terminal = true;
                    self.check(&guard)?;
                    return if status.code() == Some(i32::from(SUCCESS)) {
                        Ok(())
                    } else {
                        Err(failed())
                    };
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })();
        operation.map_err(|error| self.abort(error))
    }
    fn check(&self, guard: &impl Fn() -> Result<()>) -> Result<()> {
        crate::core::deadline::check(self.deadline)?;
        guard()?;
        crate::core::deadline::check(self.deadline)
    }
    pub(crate) fn abort(&mut self, error: AppError) -> AppError {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        if self.terminal {
            return error;
        }
        if self.cleanup_attempted {
            return cleanup_unknown();
        }
        self.cleanup_attempted = true;
        match self.child.try_wait() {
            Ok(Some(_)) => {
                self.terminal = true;
                return error;
            }
            Ok(None) => {
                let _ = self.child.kill();
            }
            Err(_) => (), // No signal based on stale state after a query error.
        }
        let cleanup = Instant::now() + Duration::from_secs(2);
        while Instant::now() < cleanup {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                self.terminal = true;
                return error;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        cleanup_unknown()
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.terminal && !self.cleanup_attempted {
            let _ = self.abort(failed());
        }
    }
}

pub fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return None;
    }
    let (Some(budget), None) = (args.next(), args.next()) else {
        return Some(FAILURE);
    };
    let Some(ms) = budget
        .to_str()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| (1..=1_800_000).contains(v))
    else {
        return Some(FAILURE);
    };
    let Ok(fd) = rustix::io::dup(std::io::stdin().as_fd()) else {
        return Some(FAILURE);
    };
    let stream = UnixStream::from(fd);
    Some(run(stream, Duration::from_millis(ms)))
}

fn run(mut stream: UnixStream, duration: Duration) -> u8 {
    let deadline = Instant::now() + duration;
    if watchdogs(&stream, duration).is_err() {
        return FAILURE;
    }
    loop {
        let Ok((mode, descriptors)) = wire::receive(&stream) else {
            return FAILURE;
        };
        if Instant::now() >= deadline {
            return TIMEOUT;
        }
        if mode == 0 {
            return SUCCESS;
        }
        let source = descriptors[0].as_fd();
        let destination = descriptors[1].as_fd();
        if wire::validate(source, destination, mode == 2).is_err()
            || crate::native::macos_copy::node(source, destination, mode == 2, deadline).is_err()
        {
            return FAILURE;
        }
        drop(descriptors);
        if stream.write_all(&[SUCCESS]).is_err() {
            return FAILURE;
        }
    }
}

fn watchdogs(stream: &UnixStream, duration: Duration) -> std::result::Result<(), ()> {
    let monitor = stream.try_clone().map_err(|_| ())?;
    std::thread::Builder::new()
        .name("copy-parent".into())
        .spawn(move || {
            loop {
                // PEEK never consumes request bytes or ancillary descriptor rights.
                match rustix::net::recv(
                    &monitor,
                    &mut [0_u8; 1],
                    rustix::net::RecvFlags::PEEK | rustix::net::RecvFlags::DONTWAIT,
                ) {
                    Ok((0, _)) => std::process::exit(i32::from(FAILURE)),
                    Err(error)
                        if error != rustix::io::Errno::AGAIN
                            && error != rustix::io::Errno::INTR =>
                    {
                        std::process::exit(i32::from(FAILURE))
                    }
                    _ => (),
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        })
        .map_err(|_| ())?;
    std::thread::Builder::new()
        .name("copy-deadline".into())
        .spawn(move || {
            std::thread::sleep(duration);
            std::process::exit(i32::from(TIMEOUT));
        })
        .map_err(|_| ())?;
    Ok(())
}
fn failed() -> AppError {
    AppError::new(
        "MACOS_COPY_FAILED",
        "Copy worker failed; partial files were not accepted.",
        3,
    )
}
fn timeout() -> AppError {
    AppError::new("DEADLINE", "Operation deadline exceeded.", 3)
}
fn cleanup_unknown() -> AppError {
    AppError::new(
        "COPY_WORKER_CLEANUP_UNKNOWN",
        "Copy worker exit was not confirmed; no copy was accepted.",
        3,
    )
}

#[cfg(test)]
pub(crate) mod tests;

//! Shared mount-worker lifetime protocol; no disk cleanup is inferred from exit.
use super::*;
use std::{process::Child, time::Duration};
pub(super) const FINISHED: i32 = 113;
pub(super) const FAILED: i32 = 114;
pub(super) const EXPIRED: i32 = 115;
pub(super) const UNKNOWN: i32 = 116;
pub(super) struct Supervisor(pub(super) Option<Child>);
impl Supervisor {
    pub(super) fn wait(&mut self, deadline: Instant, guard: &dyn Fn() -> Result<()>) -> Result<()> {
        loop {
            if let Err(error) = crate::core::deadline::check(deadline).and_then(|_| guard()) {
                return Err(self.cancel(error));
            }
            match self.0.as_mut().ok_or_else(invalid)?.try_wait() {
                Ok(Some(status)) => {
                    self.0.take();
                    return match status.code() {
                        Some(FINISHED) => {
                            crate::core::deadline::check(deadline)?;
                            guard()
                        }
                        Some(FAILED) => Err(invalid()),
                        Some(EXPIRED) => Err(expired()),
                        _ => Err(unknown()),
                    };
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                Err(_) => return Err(self.cancel(unknown())),
            }
        }
    }
    pub(super) fn cancel(&mut self, cause: AppError) -> AppError {
        let Some(mut child) = self.0.take() else {
            return cause;
        };
        child.stdin.take();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if matches!(status.code(), Some(FINISHED | FAILED | EXPIRED)) {
                        cause
                    } else {
                        unknown()
                    };
                }
                Err(_) => return unknown(),
                Ok(None) if Instant::now() >= deadline => return unknown(),
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
pub(super) fn invalid() -> AppError {
    AppError::new(
        "QQ_MOUNT_EFFECT_FAILED",
        "Mount effect was not confirmed; retain the intent and scene without replaying.",
        3,
    )
}
pub(super) fn expired() -> AppError {
    AppError::new("DEADLINE", "Operation deadline exceeded.", 3)
}
pub(super) fn unknown() -> AppError {
    AppError::new(
        "QQ_MOUNT_EFFECT_EXIT_UNKNOWN",
        "Mount worker exit was not confirmed; retain the scene without inferring disk cleanup.",
        3,
    )
}

pub(super) struct Parent;
impl Parent {
    pub(super) fn prepare(deadline: Instant) -> Result<Self> {
        let stdin = std::io::stdin();
        let flags = rustix::fs::fcntl_getfl(&stdin).map_err(|_| invalid())?;
        rustix::fs::fcntl_setfl(&stdin, flags | OFlags::NONBLOCK).map_err(|_| invalid())?;
        loop {
            crate::core::deadline::check(deadline)?;
            let mut byte = [0u8; 1];
            match rustix::io::read(&stdin, &mut byte) {
                Ok(0) => return Err(invalid()),
                Ok(1) if byte == [1] => break,
                Ok(_) => return Err(invalid()),
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                Err(_) => return Err(invalid()),
            }
        }
        let parent = Self;
        parent.check(deadline)?;
        Ok(parent)
    }
    pub(super) fn check(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        loop {
            crate::core::deadline::check(deadline)?;
            match rustix::io::read(std::io::stdin(), &mut [0u8; 1]) {
                Err(rustix::io::Errno::AGAIN) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                _ => return Err(invalid()), // EOF or extra data revokes permission.
            }
        }
    }
}

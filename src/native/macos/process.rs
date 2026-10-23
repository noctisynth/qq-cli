//! Read-only process observations. A kqueue subscription is not a process-tree capability.
use crate::core::error::{AppError, Result};
use crate::native::process::Identity;
use std::cell::Cell;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Instant;

#[path = "census.rs"]
pub(crate) mod census;

pub(crate) struct Watch {
    queue: OwnedFd,
    invalid: Cell<Option<&'static str>>,
}
impl Watch {
    pub(crate) fn is_suspended(&self, expected: &Identity, deadline: Instant) -> Result<bool> {
        self.verify(expected, deadline)?;
        let status = bsd_info(expected.process_id() as i32)?.pbi_status;
        self.verify(expected, deadline)?;
        Ok(status == libc::SSTOP)
    }
    pub(crate) fn observe(pid: u32, deadline: Instant) -> Result<(Self, Identity)> {
        let before = snapshot(pid, deadline)?;
        // SAFETY: kqueue has no pointer inputs. A successful fresh descriptor is owned exactly once.
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(unavailable());
        }
        // SAFETY: fd is the newly created descriptor, not borrowed or otherwise owned.
        let queue = unsafe { OwnedFd::from_raw_fd(fd) };
        // SAFETY: F_SETFD takes an integer flag; queue stays alive throughout the call.
        if unsafe { libc::fcntl(queue.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(unavailable());
        }
        let change = libc::kevent {
            ident: pid as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_RECEIPT,
            fflags: libc::NOTE_EXIT | libc::NOTE_EXEC,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        let mut receipt = change;
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: exactly one initialized input and one writable output event; no retained pointers.
        let count =
            unsafe { libc::kevent(queue.as_raw_fd(), &change, 1, &mut receipt, 1, &timeout) };
        if count != 1 || receipt.flags & libc::EV_ERROR == 0 || receipt.data != 0 {
            return Err(unavailable());
        }
        let watch = Self {
            queue,
            invalid: Cell::new(None),
        };
        watch.verify(&before, deadline)?;
        Ok((watch, before))
    }
    pub(crate) fn verify(&self, expected: &Identity, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        self.events()?;
        let observed = snapshot(expected.process_id(), deadline)?;
        self.events()?;
        if &observed != expected {
            self.invalid.set(Some("PROCESS_IDENTITY_CHANGED"));
            return Err(failure("PROCESS_IDENTITY_CHANGED"));
        }
        crate::core::deadline::check(deadline)
    }
    fn events(&self) -> Result<()> {
        if let Some(code) = self.invalid.get() {
            return Err(failure(code));
        }
        // SAFETY: kevent is a plain integer/pointer C structure; all-zero fields are valid output storage.
        let mut event: libc::kevent = unsafe { std::mem::zeroed() };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: no change inputs; one writable event; owned queue descriptor remains open.
        let count = unsafe {
            libc::kevent(
                self.queue.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            )
        };
        let code = if count < 0 {
            Some("PROCESS_UNVERIFIABLE")
        } else if count == 0 {
            None
        } else if event.flags & libc::EV_ERROR != 0 {
            Some("PROCESS_UNVERIFIABLE")
        } else if event.fflags & libc::NOTE_EXIT != 0 {
            Some("PROCESS_EXITED")
        } else {
            // EXEC and any unexpected notification invalidate the original capability permanently.
            Some("PROCESS_IDENTITY_CHANGED")
        };
        self.invalid.set(code);
        code.map_or(Ok(()), |code| Err(failure(code)))
    }
}
fn snapshot(pid: u32, deadline: Instant) -> Result<Identity> {
    crate::core::deadline::check(deadline)?;
    let pid_native = i32::try_from(pid)
        .ok()
        .filter(|pid| *pid > 0)
        .ok_or_else(unavailable)?;
    let first = info(pid_native)?;
    let mut path = [0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the buffer is writable for exactly its supplied size; pid is explicit, never enumerated.
    let length =
        unsafe { libc::proc_pidpath(pid_native, path.as_mut_ptr().cast(), path.len() as u32) };
    if length <= 0 || length as usize >= path.len() {
        return Err(unavailable());
    }
    let image = std::str::from_utf8(&path[..length as usize]).map_err(|_| unavailable())?;
    if image.as_bytes().contains(&0) {
        return Err(unavailable());
    }
    let second = info(pid_native)?;
    if first != second {
        return Err(failure("PROCESS_IDENTITY_CHANGED"));
    }
    let identity = Identity::macos(pid, first.0, first.1, image.into(), boot_session()?);
    identity.validate_for_host()?;
    crate::core::deadline::check(deadline)?;
    Ok(identity)
}
fn info(pid: i32) -> Result<(u64, u64)> {
    let info = bsd_info(pid)?;
    Ok((info.pbi_start_tvsec, info.pbi_start_tvusec))
}
fn bsd_info(pid: i32) -> Result<libc::proc_bsdinfo> {
    // SAFETY: proc_bsdinfo is a plain C integer/array structure, zero initialization is valid.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info);
    // SAFETY: correct SDK flavor and exact output structure size, writable for the duration of this call.
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size as i32,
        )
    };
    if read != size as i32 || info.pbi_pid != pid as u32 {
        return Err(unavailable());
    }
    // PROC_FLAG_INEXIT is the public proc_info.h flag 4.
    if info.pbi_status == libc::SZOMB || info.pbi_flags & 4 != 0 {
        return Err(failure("PROCESS_EXITED"));
    }
    Ok(info)
}
fn boot_session() -> Result<String> {
    let mut buffer = [0_u8; 64];
    let mut size = buffer.len();
    // SAFETY: NUL-terminated constant name, bounded writable output, no new value (read-only).
    let result = unsafe {
        libc::sysctlbyname(
            c"kern.bootsessionuuid".as_ptr(),
            buffer.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    if result != 0 || size != 37 || buffer[36] != 0 {
        return Err(unavailable());
    }
    String::from_utf8(buffer[..36].to_vec()).map_err(|_| unavailable())
}
fn failure(code: &'static str) -> AppError {
    AppError::new(
        code,
        "Original process observation is no longer valid; no replacement was adopted.",
        4,
    )
}
fn unavailable() -> AppError {
    failure("PROCESS_UNVERIFIABLE")
}

//! Copy between held descriptors; no path lookup, recursion, execution or unlink.
use crate::core::error::{AppError, Result};
use std::ffi::{c_char, c_void};
use std::os::fd::{AsRawFd, BorrowedFd};
use std::time::Instant;

struct State(libc::copyfile_state_t);
impl Drop for State {
    fn drop(&mut self) {
        // SAFETY: owns one non-null SDK state; copy and callbacks have returned.
        unsafe { libc::copyfile_state_free(self.0) };
    }
}

unsafe extern "C" fn progress(
    _what: i32,
    stage: i32,
    _state: libc::copyfile_state_t,
    _source: *const c_char,
    _destination: *const c_char,
    context: *mut c_void,
) -> i32 {
    // SAFETY: caller keeps this Instant live until synchronous fcopyfile returns.
    let deadline = unsafe { &*context.cast::<Instant>() };
    if stage == libc::COPYFILE_ERR || Instant::now() >= *deadline {
        libc::COPYFILE_QUIT
    } else {
        libc::COPYFILE_CONTINUE
    }
}

/// Caller exclusively creates the destination and authenticates both nodes.
/// Cancellation is cooperative; this is not a hard syscall time limit.
pub(crate) fn node(
    source: BorrowedFd<'_>,
    destination: BorrowedFd<'_>,
    data: bool,
    deadline: Instant,
) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    // SAFETY: system allocator returns an independently owned opaque object.
    let pointer = unsafe { libc::copyfile_state_alloc() };
    if pointer.is_null() {
        return Err(invalid());
    }
    let state = State(pointer);
    // SAFETY: SDK stores callback and context pointers for the synchronous call
    // below. Their ABI and lifetimes match copyfile_callback_t; no unwind occurs.
    let configured = unsafe {
        libc::copyfile_state_set(
            state.0,
            libc::COPYFILE_STATE_STATUS_CB as u32,
            progress as *const () as *const c_void,
        ) == 0
            && libc::copyfile_state_set(
                state.0,
                libc::COPYFILE_STATE_STATUS_CTX as u32,
                (&deadline as *const Instant).cast(),
            ) == 0
    };
    if !configured {
        return Err(invalid());
    }
    let flags = libc::COPYFILE_METADATA | if data { libc::COPYFILE_DATA } else { 0 };
    // SAFETY: both borrowed descriptors stay live and the state/context remain
    // valid. SDK neither closes our descriptors nor retains them after return.
    let result =
        unsafe { libc::fcopyfile(source.as_raw_fd(), destination.as_raw_fd(), state.0, flags) };
    crate::core::deadline::check(deadline)?;
    if result != 0 {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> AppError {
    AppError::new(
        "MACOS_COPY_FAILED",
        "Application copy failed; partial files were not accepted.",
        3,
    )
}

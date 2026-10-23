//! Exact-file discovery does not prove ancestry, sandbox coverage, or tree quiescence.
use crate::core::error::{AppError, Result};
use crate::native::process::Pinned;
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Instant;

unsafe extern "C" {
    fn qq_cli_file_processes(path: *const libc::c_char, pids: *mut i32, bytes: i32) -> i32;
}

/// Pins a regular, canonical, non-hardlinked executable file for repeated discovery.
/// Callers must independently authenticate the inventory and restrict execution;
/// neither this object nor an empty observation grants stop/completion authority.
pub struct Executable {
    path: PathBuf,
    native_path: CString,
    file: File,
    original: Stamp,
}

#[derive(PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    size: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn read(metadata: Metadata) -> Result<Self> {
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.mode() & 0o111 == 0 {
            return Err(unavailable());
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            size: metadata.size(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        })
    }
}
impl Executable {
    pub fn open(path: &Path, deadline: Instant) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        if !path.is_absolute() || path.canonicalize().map_err(|_| unavailable())? != path {
            return Err(unavailable());
        }
        let native_path =
            CString::new(path.as_os_str().as_encoded_bytes()).map_err(|_| unavailable())?;
        let file = File::options()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| unavailable())?;
        let original = Stamp::read(file.metadata().map_err(|_| unavailable())?)?;
        let executable = Self {
            path: path.into(),
            native_path,
            file,
            original,
        };
        executable.verify(deadline)?;
        Ok(executable)
    }

    /// Queries at most 4096 file-reference candidates and filters by actual image.
    /// Any unobservable candidate or saturated buffer fails the entire observation.
    /// This is a changing snapshot, never an atomic or complete process-tree proof.
    pub fn candidates(&self, deadline: Instant) -> Result<Vec<Pinned>> {
        self.verify(deadline)?;
        let mut pids = [0_i32; 4096];
        let capacity = std::mem::size_of_val(&pids) as i32;
        // SAFETY: NUL-terminated retained path and writable array with exact byte size;
        // SDK adapter retains no pointers and returns bytes, not an element count.
        let bytes = unsafe {
            qq_cli_file_processes(self.native_path.as_ptr(), pids.as_mut_ptr(), capacity)
        };
        if bytes < 0 || bytes >= capacity || !(bytes as usize).is_multiple_of(size_of::<i32>()) {
            return Err(unavailable());
        }
        let count = bytes as usize / size_of::<i32>();
        let mut observed = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for &pid in &pids[..count] {
            if pid <= 0 || !seen.insert(pid) {
                return Err(unavailable());
            }
            let process = Pinned::observe(pid as u32, deadline)?;
            if process.identity().image() == self.path {
                observed.push(process);
            }
        }
        self.verify(deadline)?;
        for process in &observed {
            process.verify(deadline)?;
        }
        Ok(observed)
    }

    pub fn verify(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        if self.path.canonicalize().map_err(|_| unavailable())? != self.path
            || Stamp::read(self.file.metadata().map_err(|_| unavailable())?)? != self.original
            || Stamp::read(std::fs::symlink_metadata(&self.path).map_err(|_| unavailable())?)?
                != self.original
        {
            return Err(unavailable());
        }
        crate::core::deadline::check(deadline)
    }
}
fn unavailable() -> AppError {
    AppError::new(
        "PROCESS_CENSUS_UNVERIFIABLE",
        "Executable-file observation could not be verified; no ownership or exit was inferred.",
        4,
    )
}

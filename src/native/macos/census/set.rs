//! Current-UID census with a stable birth epoch. No ancestry or signal authority.
use super::*;
use std::{collections::BTreeMap, ffi::OsString, os::unix::ffi::OsStringExt};
unsafe extern "C" {
    fn qq_cli_user_processes(uid: u32, pids: *mut i32, bytes: i32) -> i32;
}
pub struct ExecutableSet {
    root: PathBuf,
    directory: File,
    identity: (u64, u64),
    files: BTreeMap<PathBuf, Executable>,
}
impl ExecutableSet {
    pub fn open(root: &Path, paths: &[PathBuf], deadline: Instant) -> Result<Self> {
        if paths.is_empty()
            || paths.len() > 512
            || !root.is_absolute()
            || root.canonicalize().map_err(|_| unavailable())? != root
        {
            return Err(unavailable());
        }
        let directory = File::options()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)
            .map_err(|_| unavailable())?;
        let meta = directory.metadata().map_err(|_| unavailable())?;
        let mut files = BTreeMap::new();
        for path in paths {
            if !path.starts_with(root) || files.contains_key(path) {
                return Err(unavailable());
            }
            files.insert(path.clone(), Executable::open(path, deadline)?);
        }
        let scope = Self {
            root: root.into(),
            directory,
            identity: (meta.dev(), meta.ino()),
            files,
        };
        scope.verify(deadline)?;
        Ok(scope)
    }
    pub fn verify(&self, deadline: Instant) -> Result<()> {
        crate::core::deadline::check(deadline)?;
        let held = self.directory.metadata().map_err(|_| unavailable())?;
        let current = std::fs::symlink_metadata(&self.root).map_err(|_| unavailable())?;
        if !current.is_dir()
            || (current.dev(), current.ino()) != self.identity
            || (held.dev(), held.ino()) != self.identity
            || self.root.canonicalize().map_err(|_| unavailable())? != self.root
        {
            return Err(unavailable());
        }
        for file in self.files.values() {
            file.verify(deadline)?;
        }
        Ok(())
    }
    /// Only matching executable identities leave this method. Unrelated image
    /// paths and birth observations are transient and never serialized/output.
    pub fn candidates(&self, deadline: Instant) -> Result<Vec<Pinned>> {
        self.verify(deadline)?;
        let processes = live_processes(deadline)?;
        let mut candidates = Vec::new();
        for info in &processes {
            crate::core::deadline::check(deadline)?;
            let pid = info.pbi_pid as i32;
            let mut bytes = [0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
            // SAFETY: explicit current-UID PID, bounded writable SDK path buffer.
            let count =
                unsafe { libc::proc_pidpath(pid, bytes.as_mut_ptr().cast(), bytes.len() as u32) };
            if count <= 0 {
                let error = std::io::Error::last_os_error().raw_os_error();
                // XNU resolves the text vnode back to a linked pathname. ENOENT
                // cannot identify any held inventory file: each is still linked
                // at its canonical, non-hardlinked path with its original stamp.
                // This excludes an image from this file set, not process exit or
                // ownership. Access errors and all other failures remain unknown.
                if error == Some(libc::ENOENT) {
                    self.verify(deadline)?;
                    continue;
                }
                let code = match error {
                    Some(libc::EPERM) => "PROCESS_CENSUS_IMAGE_DENIED",
                    Some(libc::ESRCH) => "PROCESS_CENSUS_IMAGE_EXITED",
                    Some(libc::EINVAL) => "PROCESS_CENSUS_IMAGE_INVALID",
                    _ => "PROCESS_CENSUS_IMAGE_UNVERIFIABLE",
                };
                return Err(census_error(code));
            }
            if count as usize >= bytes.len() {
                return Err(census_error("PROCESS_CENSUS_IMAGE_TRUNCATED"));
            }
            let path = PathBuf::from(OsString::from_vec(bytes[..count as usize].to_vec()));
            if path.starts_with(&self.root) {
                if !self.files.contains_key(&path) || candidates.len() >= 256 {
                    return Err(census_error("PROCESS_CENSUS_IMAGE_OUTSIDE_INVENTORY"));
                }
                let owner = Pinned::observe(pid as u32, deadline)?;
                if owner.identity().image() != path {
                    return Err(census_error("PROCESS_CENSUS_IMAGE_CHANGED"));
                }
                candidates.push(owner);
            }
        }
        if epoch(&processes) != epoch(&live_processes(deadline)?) {
            return Err(census_error("PROCESS_CENSUS_EPOCH_CHANGED"));
        }
        self.verify(deadline)?;
        for owner in &candidates {
            owner.verify(deadline)?;
        }
        Ok(candidates)
    }
}
fn user_pids() -> Result<Vec<i32>> {
    let mut pids = [0_i32; 4096];
    let capacity = std::mem::size_of_val(&pids) as i32;
    // SAFETY: bounded writable array, current effective UID only, no retained pointers.
    let bytes = unsafe {
        qq_cli_user_processes(
            rustix::process::geteuid().as_raw(),
            pids.as_mut_ptr(),
            capacity,
        )
    };
    if bytes <= 0 || bytes >= capacity || !(bytes as usize).is_multiple_of(size_of::<i32>()) {
        return Err(census_error("PROCESS_CENSUS_LIST_INVALID"));
    }
    let mut result = pids[..bytes as usize / size_of::<i32>()].to_vec();
    result.sort_unstable();
    if result.iter().any(|p| *p <= 0) || result.windows(2).any(|v| v[0] == v[1]) {
        return Err(census_error("PROCESS_CENSUS_PID_INVALID"));
    }
    Ok(result)
}

fn census_error(code: &'static str) -> AppError {
    AppError::new(
        code,
        "Process inventory observation did not converge; no ownership or completion was inferred.",
        4,
    )
}

// proc_listpids can retain zombies which PROC_PIDTBSDINFO reports as ESRCH.
// Only that kernel absence and explicit terminal states are excluded. Compare
// live birth generations, rather than requiring dead PID slots to be readable.
fn live_processes(deadline: Instant) -> Result<Vec<libc::proc_bsdinfo>> {
    let mut result = Vec::new();
    for pid in user_pids()? {
        crate::core::deadline::check(deadline)?;
        let Some(info) = super::super::raw_bsd_info(pid)? else {
            continue;
        };
        if info.pbi_uid != rustix::process::geteuid().as_raw() {
            return Err(census_error("PROCESS_CENSUS_UID_CHANGED"));
        }
        if info.pbi_status != libc::SZOMB && info.pbi_flags & 4 == 0 {
            result.push(info);
        }
    }
    Ok(result)
}
fn epoch(processes: &[libc::proc_bsdinfo]) -> Vec<(u32, u64, u64)> {
    processes
        .iter()
        .map(|p| (p.pbi_pid, p.pbi_start_tvsec, p.pbi_start_tvusec))
        .collect()
}

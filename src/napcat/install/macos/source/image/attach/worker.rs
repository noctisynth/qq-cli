use super::*;
use std::{ffi::OsStr, os::fd::AsFd, time::Duration};

pub(super) fn verify_inputs(image: &Path, mount: &Path, input: &File, root: &File) -> Result<()> {
    acquire::private_file(input)?;
    if stamp(input)? != stamp(&open(image)?)? {
        return Err(invalid());
    }
    let current = acquire::directory(mount)?;
    let original = root.metadata().map_err(|_| invalid())?;
    let now = current.metadata().map_err(|_| invalid())?;
    if !original.is_dir()
        || (original.dev(), original.ino()) != (now.dev(), now.ino())
        || mount.canonicalize().map_err(|_| invalid())? != mount
        || std::fs::read_dir(mount)
            .map_err(|_| invalid())?
            .next()
            .is_some()
    {
        return Err(invalid());
    }
    crate::native::macos_acl::require_absent(root.as_fd())?;
    Ok(())
}
/// Private mode: stdin permission/liveness, stdout image FD, stderr mount FD.
/// Never emits output and never detaches, including on timeout or parent death.
pub(in crate::napcat::install::macos) fn worker_entry() -> Option<u8> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(FLAG)) {
        return None;
    }
    let result = (|| {
        let (Some(image), Some(mount), Some(budget), None) =
            (args.next(), args.next(), args.next(), args.next())
        else {
            return Err(invalid());
        };
        let ms = budget
            .to_str()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|v| (1..=300_000).contains(v))
            .ok_or_else(invalid)?;
        let deadline = Instant::now() + Duration::from_millis(ms);
        let parent = Parent::prepare(deadline)?;
        let input = File::from(rustix::io::dup(std::io::stdout()).map_err(|_| invalid())?);
        let root = File::from(rustix::io::dup(std::io::stderr()).map_err(|_| invalid())?);
        for file in [&input, &root] {
            rustix::io::fcntl_setfd(file, rustix::io::FdFlags::CLOEXEC).map_err(|_| invalid())?;
        }
        let image = Path::new(&image);
        let mount = Path::new(&mount);
        verify_inputs(image, mount, &input, &root)?;
        let guard = || parent.check(deadline);
        mounted::require_absent(image, mount, deadline, &guard)?;
        verify_inputs(image, mount, &input, &root)?;
        guard()?;
        mounted::command::run(
            "/usr/bin/hdiutil",
            &[
                OsStr::new("attach"),
                OsStr::new("-readonly"),
                OsStr::new("-nobrowse"),
                OsStr::new("-noautoopen"),
                OsStr::new("-mountpoint"),
                mount.as_os_str(),
                OsStr::new("-plist"),
                image.as_os_str(),
            ],
            None,
            deadline,
            &guard,
        )
        .map(|_| ())
    })();
    Some(match result {
        Ok(()) => FINISHED,
        Err(e) if e.code == "MOUNT_QUERY_EXIT_UNKNOWN" => UNKNOWN,
        Err(e) if e.code == "DEADLINE" => EXPIRED,
        Err(_) => FAILED,
    } as u8)
}

#[cfg(test)]
mod tests;

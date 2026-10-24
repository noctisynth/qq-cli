//! Preserve source quarantine before copying any recoverable image bytes.
use super::*;
use std::os::fd::AsFd;

pub(super) fn read(file: &File) -> Result<Option<Vec<u8>>> {
    let mut value = vec![0; 65536];
    match crate::native::macos_xattr::read(file.as_fd(), c"com.apple.quarantine", &mut value) {
        Ok(count) if count <= value.len() => {
            value.truncate(count);
            Ok(Some(value))
        }
        Err(error) if error.raw_os_error() == Some(libc::ENOATTR) => Ok(None),
        _ => Err(invalid()),
    }
}

pub(super) fn copy(image: &Image, destination: &File, deadline: Instant) -> Result<()> {
    image.revalidate(deadline)?;
    if let Some(value) = &image.quarantine {
        rustix::fs::fsetxattr(
            destination,
            "com.apple.quarantine",
            value,
            rustix::fs::XattrFlags::CREATE,
        )
        .map_err(|_| invalid())?;
    }
    if read(destination)? != image.quarantine {
        return Err(invalid());
    }
    Ok(())
}

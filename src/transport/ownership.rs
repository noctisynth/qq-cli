//! Platform capabilities supplied by authenticated lifecycle code, never inferred from a URL.
use crate::core::error::Result;
use crate::native::process::Pinned;
use std::net::TcpStream;
use std::time::Instant;

pub(crate) enum Guard<'a> {
    #[cfg(windows)]
    Windows(&'a crate::native::job::Job, &'a Pinned),
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    Macos(&'a Pinned),
}
impl Guard<'_> {
    pub(crate) fn bind(&self, stream: TcpStream, deadline: Instant) -> Result<TcpStream> {
        match self {
            #[cfg(windows)]
            Self::Windows(job, owner) => job
                .bind_connected(stream, owner, deadline)
                .map(crate::native::job::BoundStream::into_stream),
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            Self::Macos(owner) => {
                crate::native::macos_socket::bind_connected(stream, owner, deadline)
            }
        }
    }
    pub(crate) fn verify(&self, deadline: Instant) -> Result<()> {
        match self {
            #[cfg(windows)]
            Self::Windows(job, owner) => {
                owner.verify(deadline)?;
                if !job.contains(owner, deadline)? {
                    return Err(crate::core::error::AppError::new(
                        "ENDPOINT_OWNER_MISMATCH",
                        "Backend process no longer matches its run.",
                        4,
                    ));
                }
                Ok(())
            }
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            Self::Macos(owner) => owner.verify(deadline),
        }
    }
}

//! Lazy artifact acquisition: trust gates run before opening any input.
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::Instant;

use reqwest::Url;
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_ENCODING, LOCATION};

use crate::catalog::Release;
use crate::error::{AppError, Result};
use crate::installation;
use crate::output::Envelope;
use crate::references::References;
use crate::state::{Store, reject_links};

/// Installation import only reads its source when no verified cached artifact exists.
pub fn install(
    release: &Release,
    from: Option<&Path>,
    store: &Store,
    references: &References,
    deadline: Instant,
) -> Result<Envelope> {
    release.require_authorization()?;
    crate::deadline::check(deadline)?;
    let mut source = LazySource {
        open: Some(|| open(release, from, deadline)),
        reader: None,
        deadline,
        error: None,
    };
    installation::import(&mut source, release, store, references, deadline)
        .map_err(|error| source.error.take().unwrap_or(error))
}

struct LazySource<F> {
    open: Option<F>,
    reader: Option<Box<dyn Read>>,
    deadline: Instant,
    error: Option<AppError>,
}
impl<F: FnOnce() -> Result<Box<dyn Read>>> Read for LazySource<F> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if Instant::now() >= self.deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
        if self.reader.is_none() {
            let open = self.open.take().ok_or(io::ErrorKind::UnexpectedEof)?;
            match open() {
                Ok(reader) => self.reader = Some(reader),
                Err(error) => {
                    self.error = Some(error);
                    return Err(io::Error::other("Artifact source unavailable"));
                }
            }
        }
        self.reader
            .as_mut()
            .ok_or(io::ErrorKind::UnexpectedEof)?
            .read(buffer)
    }
}

fn open(release: &Release, from: Option<&Path>, deadline: Instant) -> Result<Box<dyn Read>> {
    crate::deadline::check(deadline)?;
    if let Some(path) = from {
        reject_links(path)?;
        let file = File::open(path).map_err(|_| unavailable())?;
        let metadata = file.metadata().map_err(|_| unavailable())?;
        if !metadata.is_file() || metadata.len() != release.archive.bytes {
            return Err(unavailable());
        }
        return Ok(Box::new(file));
    }
    Ok(Box::new(download(release, deadline)?))
}

fn download(release: &Release, deadline: Instant) -> Result<Response> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(unavailable)?;
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(remaining)
        .connect_timeout(remaining.min(std::time::Duration::from_secs(5)))
        .build()
        .map_err(|_| unavailable())?;
    let mut url = Url::parse(release.url).map_err(|_| unavailable())?;
    validate_url(&url, true)?;
    for hop in 0..=3 {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(unavailable)?;
        let response = client
            .get(url.clone())
            .timeout(remaining)
            .send()
            .map_err(|_| unavailable())?;
        if response.status().is_redirection() {
            if hop == 3 {
                return Err(unavailable());
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(unavailable)?;
            url = url.join(location).map_err(|_| unavailable())?;
            validate_url(&url, false)?;
            continue;
        }
        if response.status() != reqwest::StatusCode::OK
            || response
                .content_length()
                .is_some_and(|size| size != release.archive.bytes)
            || response
                .headers()
                .get(CONTENT_ENCODING)
                .is_some_and(|v| v != "identity")
        {
            return Err(unavailable());
        }
        return Ok(response);
    }
    Err(unavailable())
}

fn validate_url(url: &Url, initial: bool) -> Result<()> {
    let host = url.host_str().ok_or_else(unavailable)?;
    if url.as_str().len() > 8192
        || url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || if initial {
            host != "github.com"
        } else {
            !matches!(host, "github.com" | "release-assets.githubusercontent.com")
        }
    {
        return Err(unavailable());
    }
    Ok(())
}
fn unavailable() -> AppError {
    AppError::new(
        "ARTIFACT_SOURCE_UNAVAILABLE",
        "Artifact source could not be read within the fixed source and transfer limits.",
        6,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artifact_redirect_policy_rejects_other_origins_and_credentials() {
        for url in [
            "http://github.com/file",
            "https://github.com:444/file",
            "https://user@github.com/file",
            "https://github.com/file#fragment",
            "https://github.com.evil.invalid/file",
            "https://127.0.0.1/file",
        ] {
            assert!(validate_url(&Url::parse(url).unwrap(), false).is_err());
        }
        assert!(validate_url(&Url::parse("https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.38/NapCat.Framework.zip").unwrap(), true).is_ok());
        let asset =
            Url::parse("https://release-assets.githubusercontent.com/original?signature=synthetic")
                .unwrap();
        assert!(validate_url(&asset, false).is_ok());
        assert!(validate_url(&asset, true).is_err());
    }
    #[test]
    fn lazy_source_opens_once_and_never_retries_a_failed_source() {
        let mut source = LazySource {
            open: Some(|| -> Result<Box<dyn Read>> { Err(unavailable()) }),
            reader: None,
            deadline: Instant::now() + std::time::Duration::from_secs(1),
            error: None,
        };
        assert_eq!(source.read(&mut []).unwrap(), 0);
        assert!(source.open.is_some());
        assert!(source.read(&mut [0]).is_err());
        assert!(source.open.is_none());
        assert!(source.read(&mut [0]).is_err());
    }
}

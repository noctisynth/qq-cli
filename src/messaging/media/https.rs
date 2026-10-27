//! Dedicated image transport: no OneBot credentials, proxies, redirects or retries.
use crate::core::error::{AppError, Result};
use reqwest::blocking::{Client, ClientBuilder};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Content {
    pub reader: Box<dyn Read>,
    pub size: Option<u64>,
}
pub trait Images {
    fn open(&self, url: &str, identity: &str, deadline: Instant) -> Result<Content>;
}
pub struct Native;
impl Images for Native {
    fn open(&self, url: &str, identity: &str, deadline: Instant) -> Result<Content> {
        validate(url, identity, deadline)?;
        let client = builder().build().map_err(|_| transport())?;
        fetch(&client, url, identity, deadline)
    }
}
fn builder() -> ClientBuilder {
    Client::builder()
        .no_proxy()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(5))
}
fn validate(url: &str, identity: &str, deadline: Instant) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    if crate::messaging::media::source::image_identity(url).as_deref() != Some(identity) {
        return Err(AppError::new(
            "MEDIA_BINDING_UNVERIFIED",
            "Image source does not match its verified resource identity.",
            4,
        ));
    }
    Ok(())
}
fn fetch(client: &Client, url: &str, identity: &str, deadline: Instant) -> Result<Content> {
    validate(url, identity, deadline)?;
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(transport)?;
    let mut response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "image/*, application/octet-stream")
        .timeout(remaining)
        .send()
        .map_err(|_error| {
            #[cfg(test)]
            eprintln!("Synthetic HTTPS failure: {:?}", _error.without_url());
            transport()
        })?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(AppError::new(
            "MEDIA_HTTP_REJECTED",
            "Image source did not return a final successful content response.",
            4,
        ));
    }
    let headers = response.headers();
    if headers
        .get(reqwest::header::CONTENT_ENCODING)
        .is_some_and(|value| value.as_bytes() != b"identity")
    {
        return Err(AppError::new(
            "MEDIA_HTTP_INVALID",
            "Image source returned an unsupported encoded body.",
            4,
        ));
    }
    if let Some(kind) = headers.get(reqwest::header::CONTENT_TYPE) {
        let kind = kind
            .to_str()
            .map_err(|_| invalid())?
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !(kind.starts_with("image/") || kind == "application/octet-stream") {
            return Err(invalid());
        }
    }
    let size = response.content_length();
    if size.is_some_and(|size| size == 0 || size > crate::messaging::media::MAX_DOWNLOAD_BYTES) {
        return Err(invalid());
    }
    let mut first = [0];
    response.read_exact(&mut first).map_err(|_| transport())?;
    crate::core::deadline::check(deadline)?;
    Ok(Content {
        reader: Box::new(std::io::Cursor::new(first).chain(response)),
        size,
    })
}
fn transport() -> AppError {
    AppError::new(
        "MEDIA_TRANSPORT",
        "Image transport failed or exceeded its deadline; no output was published.",
        3,
    )
}
fn invalid() -> AppError {
    AppError::new(
        "MEDIA_HTTP_INVALID",
        "Image source returned an unsupported content response or size.",
        4,
    )
}

#[cfg(test)]
mod tests;

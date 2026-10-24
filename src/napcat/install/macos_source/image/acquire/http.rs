use super::*;
use reqwest::{
    Url,
    blocking::{Client, Response},
    header::{ACCEPT_ENCODING, CONTENT_ENCODING, LOCATION},
};
fn allowed(url: &Url) -> Result<()> {
    if url.as_str().len() > 8192
        || url.scheme() != "https"
        || url.host_str() != Some("qqdl.gtimg.cn")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    Ok(())
}
fn builder(deadline: Instant) -> Result<reqwest::blocking::ClientBuilder> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(invalid)?;
    Ok(Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(remaining)
        .connect_timeout(remaining.min(std::time::Duration::from_secs(5))))
}
pub(super) fn download(deadline: Instant) -> Result<Response> {
    let client = builder(deadline)?.build().map_err(|_| invalid())?;
    let url = Url::parse(crate::napcat::install::macos_source::DMG_URL).map_err(|_| invalid())?;
    fetch(
        &client,
        url,
        crate::napcat::install::macos_source::DMG_BYTES,
        deadline,
    )
}
fn fetch(client: &Client, mut url: Url, bytes: u64, deadline: Instant) -> Result<Response> {
    let remaining = || {
        deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(invalid)
    };
    for hop in 0..=3 {
        allowed(&url)?;
        let response = client
            .get(url.clone())
            .header(ACCEPT_ENCODING, "identity")
            .timeout(remaining()?)
            .send()
            .map_err(|_| invalid())?;
        if response.status().is_redirection() {
            if hop == 3 {
                return Err(invalid());
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(invalid)?;
            url = url.join(location).map_err(|_| invalid())?;
            continue;
        }
        if response.status() != reqwest::StatusCode::OK
            || response.content_length().is_some_and(|v| v != bytes)
            || response
                .headers()
                .get(CONTENT_ENCODING)
                .is_some_and(|v| v != "identity")
        {
            return Err(invalid());
        }
        return Ok(response);
    }
    Err(invalid())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redirects_cannot_change_origin_or_add_credentials() {
        allowed(&Url::parse(crate::napcat::install::macos_source::DMG_URL).unwrap()).unwrap();
        for url in [
            "http://qqdl.gtimg.cn/file",
            "https://qqdl.gtimg.cn:444/file",
            "https://user@qqdl.gtimg.cn/file",
            "https://qqdl.gtimg.cn/file#fragment",
            "https://qqdl.gtimg.cn.evil.invalid/file",
            "https://127.0.0.1/file",
        ] {
            assert!(allowed(&Url::parse(url).unwrap()).is_err());
        }
    }
}

#[cfg(test)]
#[path = "tls_tests.rs"]
mod tls_tests;

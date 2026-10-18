//! Fixed-release image resource identities. URL credentials never enter references.
use reqwest::Url;
use sha2::{Digest, Sha256};

/// A missing/local URL can be resolved by re-reading the original message.
/// This metadata hint never authorizes reading that local path or fetching a URL.
pub fn can_resolve_image(raw: Option<&str>) -> bool {
    let Some(raw) = raw else { return true };
    if raw.len() > 16 * 1024 || raw.chars().any(char::is_control) {
        return false;
    }
    if image_identity(raw).is_some() {
        return true;
    }
    let bytes = raw.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\');
    drive || (!raw.contains(':') && !raw.starts_with("//") && !raw.starts_with("\\\\"))
}

/// Return a stable, non-secret identity digest for an audited image URL shape.
/// This validates a source descriptor only; it does not authorize or perform I/O.
pub fn image_identity(raw: &str) -> Option<String> {
    if raw.len() > 16 * 1024
        || raw
            .bytes()
            .any(|b| b.is_ascii_control() || b.is_ascii_whitespace() || b == b'\\')
    {
        return None;
    }
    let host = ["gchat.qpic.cn", "multimedia.nt.qq.com.cn", "gxh.vip.qq.com"]
        .into_iter()
        .find(|host| raw.starts_with(&format!("https://{host}/")))?;
    let url = Url::parse(raw).ok()?;
    if url.scheme() != "https"
        || url.host_str() != Some(host)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let path = url.path();
    if raw
        .strip_prefix(&format!("https://{host}"))?
        .split('?')
        .next()?
        != path
    {
        return None;
    }
    let identity = if host == "gxh.vip.qq.com" {
        if url.query().is_some() {
            return None;
        }
        let resource = path
            .strip_prefix("/club/item/parcel/item/")?
            .strip_suffix("/raw300.gif")?;
        let (directory, emoji) = resource.split_once('/')?;
        if emoji.len() < 2
            || emoji.len() > 128
            || !emoji
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            || directory != &emoji[..2]
        {
            return None;
        }
        format!("{host}/emoji/{emoji}")
    } else if path == "/download" {
        let mut appid = None;
        let mut fileid = None;
        let mut rkey = None;
        let mut spec = None;
        for (key, value) in url.query_pairs() {
            let target = match key.as_ref() {
                "appid" => &mut appid,
                "fileid" => &mut fileid,
                "rkey" => &mut rkey,
                "spec" if host == "gchat.qpic.cn" => &mut spec,
                _ => return None,
            };
            if target.replace(value.into_owned()).is_some() {
                return None;
            }
        }
        let appid = appid?;
        let fileid = fileid?;
        let rkey = rkey?;
        if !matches!(appid.as_str(), "1406" | "1407")
            || !token(&fileid, 4096)
            || !token(&rkey, 4096)
            || spec.as_ref().is_some_and(|s| s != "0")
        {
            return None;
        }
        // JSON framing prevents delimiters in identifiers from aliasing fields.
        serde_json::to_string(&(host, appid, fileid)).ok()?
    } else if host == "gchat.qpic.cn" {
        if url.query().is_some() {
            return None;
        }
        let md5 = path
            .strip_prefix("/gchatpic_new/0/0-0-")?
            .strip_suffix("/0")?;
        if !hex(md5, 32) {
            return None;
        }
        format!("{host}/md5/{}", md5.to_ascii_lowercase())
    } else {
        return None;
    };
    Some(format!("{:x}", Sha256::digest(identity.as_bytes())))
}

fn token(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-=+/".contains(&b))
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_hexdigit())
}

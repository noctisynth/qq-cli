//! Reviewed upstream identifiers, never an implicit `latest` or user-supplied trust manifest.
use crate::core::error::{AppError, Result};
use crate::napcat::install::archive::Expected;

pub struct Release {
    pub version: &'static str,
    pub commit: &'static str,
    pub url: &'static str,
    pub archive: Expected<'static>,
    /// Informational provenance annotation; neither legal permission nor launch authority.
    pub authorization_evidence: Option<&'static str>,
}

mod pins;

pub fn lookup(version: &str) -> Result<Release> {
    let pin = match version.strip_prefix('v').unwrap_or(version) {
        "4.18.41" => pins::V4_18_41,
        "4.18.38" => pins::V4_18_38,
        "4.18.37" => pins::V4_18_37,
        _ => {
            return Err(AppError::new(
                "VERSION_UNSUPPORTED",
                "Version is not present in the reviewed artifact catalog; latest and arbitrary versions are not accepted.",
                4,
            ));
        }
    };
    Ok(Release {
        version: pin.version,
        commit: pin.commit,
        url: pin.url,
        archive: Expected {
            bytes: pin.bytes,
            sha256: pin.sha256,
            files: pin.files,
            required: pin.required,
        },
        // FACT-002: unchanged custom license; download and provenance do not establish permission.
        authorization_evidence: None,
    })
}

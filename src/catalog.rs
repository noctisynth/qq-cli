//! Reviewed upstream identifiers, never an implicit `latest` or user-supplied trust manifest.
use crate::archive::Expected;
use crate::error::{AppError, Result};

pub struct Release {
    pub version: &'static str,
    pub commit: &'static str,
    pub url: &'static str,
    pub archive: Expected<'static>,
    pub authorization_evidence: Option<&'static str>,
}

pub fn lookup(version: &str) -> Result<Release> {
    if version.strip_prefix('v').unwrap_or(version) != "4.18.38" {
        return Err(AppError::new(
            "VERSION_UNSUPPORTED",
            "Version is not present in the reviewed artifact catalog; latest and arbitrary versions are not accepted.",
            4,
        ));
    }
    Ok(Release {
        version: "v4.18.38",
        commit: "bbd0d2aca3d470c20b33b1d4f95123335633bf26",
        url: "https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.38/NapCat.Framework.zip",
        archive: Expected {
            bytes: 36_728_573,
            sha256: "45ef7f46aa78588c636044b6226b4b1847e31287ab6ec36b2d48adaa9785097b",
            files: 680,
            required: &[
                (
                    "napimain.exe",
                    "116a7da5a0887244a4946472062a0cb0b866e353924808b659ff7d109b5ffe69",
                ),
                (
                    "napiloader.dll",
                    "f7dba30fc165ac013e4b9114df851d4c7dbb7853d05175df5050c7886e9d5311",
                ),
                (
                    "nativeLoader.cjs",
                    "98e12440f27444c686de8087bd69381bb3f3e7e567870a90507f5976e0b3e38f",
                ),
                (
                    "napcat.cjs",
                    "0bacde790aa4081b3a2d3b18faae3dee39f43b44d9fe1f622602fd2604a74887",
                ),
                (
                    "napcat.mjs",
                    "f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a",
                ),
                (
                    "native/napi2native/napi2native.win32.x64.node",
                    "46780f9cc420a840318179b26cbb551357d2ae986dd8df960ef1a49656181fcc",
                ),
                (
                    "native/packet/MoeHoo.win32.x64.node",
                    "fb62bff4d093b892bbe5c5dc4197a226881689e43f6d438a66895fa7cdbb90f1",
                ),
            ],
        },
        // FACT-002 records an unresolved custom license and missing root license in the bundle.
        // A reviewed applicable basis must be recorded before preparing a real installation.
        authorization_evidence: None,
    })
}

impl Release {
    pub fn require_authorization(&self) -> Result<()> {
        if self.authorization_evidence.is_none_or(str::is_empty) {
            return Err(AppError::new(
                "LICENSE_UNCONFIRMED",
                "Applicable NapCat integration authorization has not been established; no artifact was opened, downloaded, extracted or executed.",
                4,
            ));
        }
        Ok(())
    }
}

//! Fixed WebUI observation protocol. This is management authentication, never QQ login.
//! The backend must prove the signed run's socket ownership before sending any bytes.
#[cfg(any(windows, all(target_os = "macos", target_arch = "aarch64")))]
pub mod owned;
use crate::core::error::{AppError, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Instant;

pub const RESPONSE_LIMIT: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Authenticate,
    LoginStatus,
}
impl Route {
    pub fn path(self) -> &'static str {
        match self {
            Self::Authenticate => "/api/auth/login",
            Self::LoginStatus => "/api/QQLogin/CheckLoginStatus",
        }
    }
}

/// Production implementations must authenticate the endpoint BEFORE either the token
/// hash in the body or the server-issued credential leaves this process. No retries.
pub trait Backend {
    fn post(
        &self,
        route: Route,
        payload: &Value,
        credential: Option<&str>,
        deadline: Instant,
    ) -> Result<Vec<u8>>;
}

pub use crate::core::contracts::instance::{Observation, Phase};
impl Phase {
    fn parse(value: &str) -> Self {
        match value {
            "waiting_qrcode" => Self::WaitingQrcode,
            "generating_qrcode" => Self::GeneratingQrcode,
            "qrcode_scanned" => Self::QrcodeScanned,
            "initializing" => Self::Initializing,
            "ready" => Self::Ready,
            "offline" => Self::Offline,
            "reconnecting" => Self::Reconnecting,
            _ => Self::Unknown,
        }
    }
}

// Deliberately no Debug or Serialize: a credential is never output or persisted.
struct Credential(String);

#[derive(Deserialize)]
struct Envelope<T> {
    code: i64,
    data: Option<T>,
}
#[derive(Deserialize)]
struct Authentication {
    #[serde(rename = "Credential")]
    credential: Option<String>,
    #[serde(rename = "require2FA")]
    require_2fa: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    is_login: bool,
    is_offline: bool,
    login_phase: String,
    qr_login_accepted: bool,
    core_ready: bool,
}

/// One authentication and one status request, sharing a caller-owned total deadline.
/// Failure at any step is final; this never initiates or refreshes QQ login.
pub fn observe(backend: &impl Backend, token: &str, deadline: Instant) -> Result<Observation> {
    crate::core::deadline::check(deadline)?;
    if token.is_empty() || token.len() > 4096 || !token.bytes().all(|b| (33..127).contains(&b)) {
        return Err(AppError::new(
            "CONTROL_CREDENTIAL_INVALID",
            "Management credential is invalid.",
            2,
        ));
    }
    let hash = format!("{:x}", Sha256::digest(format!("{token}.napcat")));
    let response = backend.post(Route::Authenticate, &json!({"hash":hash}), None, deadline)?;
    crate::core::deadline::check(deadline)?;
    let authentication: Authentication = decode(&response)?;
    if authentication.require_2fa == Some(true) {
        return Err(AppError::new(
            "CONTROL_2FA_REQUIRED",
            "Management authentication requires a second factor; no login observation was attempted.",
            4,
        ));
    }
    let credential = Credential(authentication.credential.ok_or_else(protocol)?);
    if credential.0.is_empty()
        || credential.0.len() > 4096
        || !credential.0.bytes().all(|b| (33..127).contains(&b))
    {
        return Err(protocol());
    }
    let response = backend.post(
        Route::LoginStatus,
        &json!({}),
        Some(&credential.0),
        deadline,
    )?;
    crate::core::deadline::check(deadline)?;
    let status: Status = decode(&response)?;
    if status.is_login != status.core_ready || (status.is_login && status.is_offline) {
        return Err(protocol());
    }
    Ok(Observation {
        phase: Phase::parse(&status.login_phase),
        is_login: status.is_login,
        is_offline: status.is_offline,
        qr_login_accepted: status.qr_login_accepted,
        core_ready: status.core_ready,
    })
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    if bytes.len() > RESPONSE_LIMIT {
        return Err(AppError::new(
            "CONTROL_RESPONSE_LIMIT",
            "Management response exceeds its byte budget.",
            3,
        ));
    }
    let value: Envelope<T> = serde_json::from_slice(bytes).map_err(|_| protocol())?;
    if value.code != 0 {
        return Err(AppError::new(
            "CONTROL_REJECTED",
            "Management endpoint rejected the request.",
            3,
        ));
    }
    value.data.ok_or_else(protocol)
}
fn protocol() -> AppError {
    AppError::new(
        "CONTROL_PROTOCOL",
        "Management response does not match the fixed contract.",
        3,
    )
}

#[cfg(test)]
mod tests;

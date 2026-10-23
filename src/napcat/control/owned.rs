//! Fixed-route management protocol adapter. Lifecycle state supplies the pinned endpoint.
use super::{Backend, RESPONSE_LIMIT, Route};
use crate::core::error::{AppError, Result};
#[cfg(windows)]
use crate::native::job::Job;
use crate::native::process::Pinned;
use crate::storage::config::Config;
use crate::transport::owned_wire::OwnedHttp;
use crate::transport::ownership::Guard;
use serde_json::Value;
use std::time::Instant;

pub struct OwnedControl<'a> {
    wire: OwnedHttp<'a>,
    limit: usize,
}
impl<'a> OwnedControl<'a> {
    /// Inputs must originate from authenticated runtime state, never an arbitrary URL.
    /// Construction sends nothing. Each request independently proves socket ownership.
    #[cfg(windows)]
    pub fn new(job: &'a Job, owner: &'a Pinned, endpoint: &str, config: &Config) -> Result<Self> {
        Self::with_guard(Guard::Windows(job, owner), endpoint, config)
    }
    /// Lifecycle must authenticate the recorded process before supplying this capability.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    pub fn for_process(owner: &'a Pinned, endpoint: &str, config: &Config) -> Result<Self> {
        Self::with_guard(Guard::Macos(owner), endpoint, config)
    }
    fn with_guard(guard: Guard<'a>, endpoint: &str, config: &Config) -> Result<Self> {
        Ok(Self {
            wire: OwnedHttp::new(guard, endpoint, config)?,
            limit: RESPONSE_LIMIT.min(config.response_bytes),
        })
    }
}
impl Backend for OwnedControl<'_> {
    fn post(
        &self,
        route: Route,
        payload: &Value,
        credential: Option<&str>,
        deadline: Instant,
    ) -> Result<Vec<u8>> {
        let valid = match route {
            Route::Authenticate => {
                credential.is_none()
                    && payload.as_object().is_some_and(|o| o.len() == 1)
                    && payload["hash"].as_str().is_some_and(|hash| {
                        hash.len() == 64
                            && hash
                                .bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    })
            }
            Route::LoginStatus => {
                credential.is_some() && payload.as_object().is_some_and(|o| o.is_empty())
            }
        };
        if !valid {
            return Err(AppError::invalid());
        }
        self.wire
            .post(route.path(), credential, payload, deadline, self.limit)
    }
}

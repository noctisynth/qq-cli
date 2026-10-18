//! OneBot protocol adapter over the shared owned HTTP transport.
use crate::core::error::{AppError, Result};
use crate::native::{job::Job, process::Pinned};
use crate::storage::config::Config;
use crate::transport::{
    client::{Action, Backend},
    owned_wire::{OwnedHttp, authorization},
};
use serde_json::Value;
use std::time::Instant;

pub struct OwnedOneBot<'a> {
    wire: OwnedHttp<'a>,
    token: String,
    limit: usize,
}
impl<'a> OwnedOneBot<'a> {
    /// No network or credentials are sent by construction.
    pub fn new(
        job: &'a Job,
        owner: &'a Pinned,
        endpoint: &str,
        token: &str,
        config: &Config,
    ) -> Result<Self> {
        let wire = OwnedHttp::new(job, owner, endpoint, config)?;
        authorization(token)?;
        Ok(Self {
            wire,
            token: token.into(),
            limit: config.response_bytes,
        })
    }
}

impl Backend for OwnedOneBot<'_> {
    fn call(&self, action: Action, payload: &Value, deadline: Instant) -> Result<Value> {
        let bytes = self.wire.post(
            &format!("/{}", action.name()),
            Some(&self.token),
            payload,
            deadline,
            self.limit,
        )?;
        let value = crate::transport::client::parse_envelope(&bytes)?;
        crate::core::deadline::check(deadline)?;
        Ok(value)
    }
    fn media_bytes(
        &self,
        action: Action,
        payload: &Value,
        deadline: Instant,
    ) -> Result<crate::transport::media::Bytes> {
        if !matches!(action, Action::Image | Action::File) {
            return Err(AppError::invalid());
        }
        let bytes = self.wire.post(
            &format!("/{}", action.name()),
            Some(&self.token),
            payload,
            deadline,
            crate::transport::media::MAX_BASE64_BYTES + self.limit,
        )?;
        let bytes = crate::transport::media::from_slice(&bytes, self.limit)?;
        crate::core::deadline::check(deadline)?;
        Ok(bytes)
    }
}

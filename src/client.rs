use std::io::Read;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::{Value, json};

use crate::config::Config;
use crate::domain::{decimal_id, identifier};
use crate::error::{AppError, Result};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Status,
    LoginInfo,
    VersionInfo,
    FriendList,
    GroupList,
    GroupInfo,
    GroupMembers,
    RecentContacts,
    Message,
    FriendHistory,
    GroupHistory,
    Forward,
    SendPrivate,
    SendGroup,
    UploadPrivateFile,
    UploadGroupFile,
    Image,
    File,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Self::Status => "get_status",
            Self::LoginInfo => "get_login_info",
            Self::VersionInfo => "get_version_info",
            Self::FriendList => "get_friend_list",
            Self::GroupList => "get_group_list",
            Self::GroupInfo => "get_group_info",
            Self::GroupMembers => "get_group_member_list",
            Self::RecentContacts => "get_recent_contact",
            Self::Message => "get_msg",
            Self::FriendHistory => "get_friend_msg_history",
            Self::GroupHistory => "get_group_msg_history",
            Self::Forward => "get_forward_msg",
            Self::SendPrivate => "send_private_msg",
            Self::SendGroup => "send_group_msg",
            Self::UploadPrivateFile => "upload_private_file",
            Self::UploadGroupFile => "upload_group_file",
            Self::Image => "get_image",
            Self::File => "get_file",
        }
    }
    pub fn is_write(self) -> bool {
        matches!(
            self,
            Self::SendPrivate | Self::SendGroup | Self::UploadPrivateFile | Self::UploadGroupFile
        )
    }
}

#[derive(Clone, Debug)]
pub struct Endpoint(String);

impl Endpoint {
    pub fn parse(input: &str) -> Result<Self> {
        let bad = || {
            AppError::new(
                "ENDPOINT_INVALID",
                "Only explicit-port literal loopback HTTP endpoints are allowed.",
                2,
            )
        };
        let rest = input.strip_prefix("http://").ok_or_else(bad)?;
        let authority = rest.strip_suffix('/').unwrap_or(rest);
        let port = authority
            .strip_prefix("127.0.0.1:")
            .or_else(|| authority.strip_prefix("[::1]:"))
            .ok_or_else(bad)?;
        if port.is_empty()
            || !port.bytes().all(|b| b.is_ascii_digit())
            || port.parse::<u16>().ok().is_none_or(|p| p == 0)
        {
            return Err(bad());
        }
        let host = if authority.starts_with('[') {
            "[::1]"
        } else {
            "127.0.0.1"
        };
        let port: u16 = port.parse().map_err(|_| bad())?;
        Ok(Self(format!("http://{host}:{port}/")))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub trait Backend {
    fn call(&self, action: Action, payload: &Value, deadline: Instant) -> Result<Value>;
    fn media_bytes(
        &self,
        action: Action,
        payload: &Value,
        deadline: Instant,
    ) -> Result<crate::media_transport::Bytes> {
        if !matches!(action, Action::Image | Action::File) {
            return Err(AppError::invalid());
        }
        crate::media_transport::from_value(self.call(action, payload, deadline)?)
    }
}

pub struct OneBotClient {
    client: Client,
    endpoint: Endpoint,
    timeout: Duration,
    limit: usize,
}

impl OneBotClient {
    pub fn new(endpoint: &str, token: &str, config: &Config) -> Result<Self> {
        let endpoint = Endpoint::parse(endpoint)?;
        if !(1024..=4 * 1024 * 1024).contains(&config.response_bytes)
            || !(1..=60).contains(&config.request_timeout_seconds)
        {
            return Err(AppError::new(
                "CONFIG_INVALID",
                "Backend transport limits are outside the supported range.",
                2,
            ));
        }
        if token.is_empty() || token.len() > 4096 {
            return Err(AppError::new(
                "CREDENTIAL_INVALID",
                "Backend credentials are invalid.",
                2,
            ));
        }
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
                AppError::new("CREDENTIAL_INVALID", "Backend credentials are invalid.", 2)
            })?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        let timeout = Duration::from_secs(u64::from(config.request_timeout_seconds));
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout)
            .connect_timeout(timeout.min(Duration::from_secs(5)))
            .default_headers(headers)
            .build()
            .map_err(|_| {
                AppError::new(
                    "HTTP_INIT",
                    "Local backend transport could not be initialized.",
                    6,
                )
            })?;
        Ok(Self {
            client,
            endpoint,
            timeout,
            limit: config.response_bytes,
        })
    }
}

impl OneBotClient {
    pub(crate) fn response_limit(&self) -> usize {
        self.limit
    }
    pub(crate) fn response(
        &self,
        action: Action,
        payload: &Value,
        deadline: Instant,
    ) -> Result<reqwest::blocking::Response> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| AppError::new("DEADLINE", "Operation deadline exceeded.", 3))?;
        let response = self
            .client
            .post(format!("{}{}", self.endpoint.as_str(), action.name()))
            .json(payload)
            .timeout(remaining.min(self.timeout))
            .send()
            .map_err(|_| {
                AppError::new(
                    "BACKEND_TRANSPORT",
                    "Backend request failed or timed out.",
                    3,
                )
            })?;
        if matches!(response.status().as_u16(), 401 | 403) {
            return Err(AppError::new(
                "BACKEND_AUTH",
                "Backend rejected authentication.",
                3,
            ));
        }
        if !response.status().is_success() {
            return Err(AppError::new(
                "BACKEND_HTTP",
                "Backend returned an unsuccessful HTTP status.",
                3,
            ));
        }
        Ok(response)
    }
}

impl Backend for OneBotClient {
    fn media_bytes(
        &self,
        action: Action,
        payload: &Value,
        deadline: Instant,
    ) -> Result<crate::media_transport::Bytes> {
        crate::media_transport::fetch(self, action, payload, deadline)
    }
    fn call(&self, action: Action, payload: &Value, deadline: Instant) -> Result<Value> {
        let response = self.response(action, payload, deadline)?;
        if response
            .content_length()
            .is_some_and(|size| size > self.limit as u64)
        {
            return Err(AppError::new(
                "RESPONSE_LIMIT",
                "Backend response exceeds its size limit.",
                3,
            ));
        }
        let mut bytes = Vec::new();
        response
            .take(self.limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                AppError::new("BACKEND_TRANSPORT", "Backend response was interrupted.", 3)
            })?;
        if bytes.len() > self.limit {
            return Err(AppError::new(
                "RESPONSE_LIMIT",
                "Backend response exceeds its size limit.",
                3,
            ));
        }
        parse_envelope(&bytes)
    }
}

pub(crate) fn parse_envelope(bytes: &[u8]) -> Result<Value> {
    let mut envelope: Value = serde_json::from_slice(bytes).map_err(|_| AppError::protocol())?;
    let status = envelope
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(AppError::protocol)?;
    let code = envelope
        .get("retcode")
        .and_then(Value::as_i64)
        .ok_or_else(AppError::protocol)?;
    final_status(status, code)?;
    envelope
        .get_mut("data")
        .map(Value::take)
        .ok_or_else(AppError::protocol)
}

pub(crate) fn final_status(status: &str, code: i64) -> Result<()> {
    if status != "ok" || code != 0 {
        if status == "async" {
            return Err(AppError::new(
                "BACKEND_ASYNC",
                "Backend did not return a final result.",
                3,
            ));
        }
        return Err(
            AppError::new("BACKEND_REJECTED", "Backend rejected the operation.", 4)
                .with_details(json!({"retcode": code})),
        );
    }
    Ok(())
}

pub struct SessionInfo {
    pub account: String,
    pub nickname: Option<String>,
    pub online: bool,
    pub version: String,
}

pub fn inspect(backend: &impl Backend, deadline: Instant) -> Result<SessionInfo> {
    let version = backend.call(Action::VersionInfo, &json!({}), deadline)?;
    let version = version
        .get("app_version")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(AppError::protocol)?
        .to_owned();
    let login = backend.call(Action::LoginInfo, &json!({}), deadline)?;
    let account = login
        .get("user_id")
        .and_then(identifier)
        .and_then(|s| decimal_id(&s).ok())
        .ok_or_else(|| {
            AppError::new(
                "AUTH_REQUIRED",
                "The selected instance has no verified login identity.",
                4,
            )
        })?;
    let status = backend.call(Action::Status, &json!({}), deadline)?;
    let online = status
        .get("online")
        .and_then(Value::as_bool)
        .ok_or_else(AppError::protocol)?;
    Ok(SessionInfo {
        account,
        nickname: login
            .get("nickname")
            .and_then(Value::as_str)
            .map(str::to_owned),
        online,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_validation_does_not_normalize_unsafe_inputs_into_valid_endpoints() {
        for bad in [
            "http://localhost:80",
            "http://127.0.0.1",
            "http://127.0.0.1:0",
            "http://127.0.0.1:80/a/..",
            "http://127.0.0.1:80?token=secret",
            "http://127.0.0.1:80/#",
            "https://127.0.0.1:80",
            "http://user@127.0.0.1:80",
        ] {
            assert!(Endpoint::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(
            Endpoint::parse("http://[::1]:16000").unwrap().as_str(),
            "http://[::1]:16000/"
        );
        assert!(Endpoint::parse("http://127.0.0.1:80/").is_ok());
    }
}

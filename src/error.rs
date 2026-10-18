use serde::Serialize;
use serde_json::Value;

pub type Result<T> = std::result::Result<T, AppError>;

/// Error messages are curated; raw backend/library errors never enter default output.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: &'static str,
    pub message: &'static str,
    pub retry_safe: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action: Option<Box<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Box<Value>>,
    #[serde(skip)]
    pub scope: Option<Box<crate::domain::Scope>>,
    #[serde(skip)]
    pub exit_code: u8,
}

impl AppError {
    pub fn new(code: &'static str, message: &'static str, exit_code: u8) -> Self {
        Self {
            code,
            message,
            retry_safe: false,
            outcome: None,
            request_id: None,
            next_action: None,
            details: None,
            scope: None,
            exit_code,
        }
    }
    pub fn invalid() -> Self {
        Self::new(
            "INVALID_ARGUMENTS",
            "Invalid arguments; inspect this command with --help.",
            2,
        )
    }
    pub fn storage() -> Self {
        Self::new(
            "STATE_IO",
            "Private local state could not be read or persisted.",
            6,
        )
    }
    pub fn protocol() -> Self {
        Self::new(
            "BACKEND_PROTOCOL",
            "Backend response does not match the supported contract.",
            3,
        )
    }
    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(Box::new(details));
        self
    }
    pub fn safe_to_retry(mut self) -> Self {
        self.retry_safe = true;
        self
    }
    pub fn with_scope(mut self, scope: crate::domain::Scope) -> Self {
        self.scope = Some(Box::new(scope));
        self
    }
}

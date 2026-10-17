use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    schema_version: u8,
    ok: bool,
    command: &'static str,
    data: Option<()>,
    error: Error,
    meta: Meta,
    #[serde(skip)]
    exit_code: u8,
}

#[derive(Serialize)]
struct Error {
    code: &'static str,
    message: &'static str,
    outcome: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    invocation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

impl Envelope {
    pub fn invalid_arguments() -> Self {
        Self::failure(
            "cli",
            "INVALID_ARGUMENTS",
            "Invalid arguments; use --help for command usage.",
            2,
            None,
        )
    }

    pub fn not_implemented(command: &'static str, request_id: Option<&str>) -> Self {
        Self::failure(
            command,
            "NOT_IMPLEMENTED",
            "This command is not implemented in M0; no operation was attempted.",
            6,
            request_id,
        )
    }

    fn failure(
        command: &'static str,
        code: &'static str,
        message: &'static str,
        exit_code: u8,
        request_id: Option<&str>,
    ) -> Self {
        Self {
            schema_version: 1,
            ok: false,
            command,
            data: None,
            error: Error {
                code,
                message,
                outcome: "not_attempted",
            },
            meta: Meta {
                invocation_id: invocation_id(),
                request_id: request_id.map(str::to_owned),
            },
            exit_code,
        }
    }

    pub fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

fn invocation_id() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("inv-{timestamp:x}-{:x}-{sequence:x}", std::process::id())
}

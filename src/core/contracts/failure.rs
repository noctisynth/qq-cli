//! Closed curated diagnostic details; raw backend payloads cannot enter errors.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Candidate {
    pub instance_id: String,
    pub state: String,
    pub account: Option<String>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged, rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Details {
    Candidates {
        candidates: Vec<Candidate>,
        truncated: bool,
    },
    Login {
        instance_id: String,
        state: LoginState,
        account: (),
        reused: bool,
    },
    Identity {
        instance_id: String,
        observed_account: String,
    },
    Instance {
        instance_id: String,
    },
    Retcode {
        retcode: i64,
    },
    InstallationUsers {
        instances: Vec<String>,
        count: usize,
    },
    Installations {
        installation_ids: Vec<String>,
    },
    QqInstallations {
        qq_installation_ids: Vec<String>,
    },
    Compatibility {
        reason: String,
        key_fingerprint: String,
        evidence: Vec<String>,
    },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub enum LoginState {
    #[serde(rename = "login_required")]
    LoginRequired,
}

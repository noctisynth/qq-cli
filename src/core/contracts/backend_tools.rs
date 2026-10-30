//! Fixed native tool payloads; opaque results stay in the explicit API boundary.
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Serialize, JsonSchema)]
pub struct EmptyPayload {}
#[derive(Serialize, JsonSchema)]
pub struct DomainPayload {
    pub domain: String,
}
#[derive(Serialize, JsonSchema)]
pub struct UrlPayload {
    pub url: String,
}
#[derive(Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Response {
    Boolean(bool),
    Text(String),
}
fn default_response() -> Response {
    Response::Boolean(true)
}
#[derive(Serialize, JsonSchema)]
pub struct PacketPayload {
    pub cmd: String,
    pub data: String,
    #[schemars(default = "default_response")]
    pub rsp: Response,
}

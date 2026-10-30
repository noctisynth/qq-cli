//! Fixed-release startup rendering; never opens files, selects paths or starts processes.
//! Secret-bearing types deliberately implement neither Debug nor Serialize.
use crate::core::error::{AppError, Result};
use serde_json::{Value, json};
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ports {
    pub http: u16,
    pub events: u16,
    pub control: u16,
}
impl Ports {
    pub fn validate(self) -> Result<()> {
        let ports = [self.http, self.events, self.control];
        if ports.iter().any(|port| *port < 1024)
            || self.http == self.events
            || self.http == self.control
            || self.events == self.control
        {
            return Err(AppError::new(
                "RUNTIME_CONFIG_INVALID",
                "Managed endpoints require three distinct non-privileged ports.",
                4,
            ));
        }
        Ok(())
    }
}

pub struct Bundle {
    version: &'static str,
    ports: Ports,
    api: String,
    control: String,
    jwt: String,
    files: Vec<ConfigFile>,
}
pub struct ConfigFile {
    name: &'static str,
    bytes: Vec<u8>,
}
impl ConfigFile {
    pub fn name(&self) -> &'static str {
        self.name
    }
    /// The caller must provide a private, owned destination before writing secrets.
    pub fn write_to(&self, mut destination: impl Write) -> Result<()> {
        destination.write_all(&self.bytes).map_err(|_| {
            AppError::new(
                "RUNTIME_CONFIG_IO",
                "Managed configuration could not be written privately.",
                6,
            )
        })
    }
}
impl Bundle {
    /// Template 1 stays byte-for-byte recoverable for existing runs. Template 2
    /// adds only original executable adapters, authenticated with the material.
    pub fn with_friend_requests(mut self, template: u8) -> Result<Self> {
        match template {
            1 => (),
            2 => {
                self.files.push(ConfigFile {
                    name: "friend_requests.cjs",
                    bytes: include_bytes!("friend_requests.cjs").to_vec(),
                });
                self.files.push(ConfigFile {
                    name: "friend_loader.cjs",
                    bytes: include_bytes!("friend_loader.cjs").to_vec(),
                });
            }
            _ => return Err(AppError::protocol()),
        }
        Ok(self)
    }
    pub fn friend_requests_enabled(&self) -> bool {
        self.files
            .iter()
            .any(|file| file.name == "friend_loader.cjs")
    }
    pub fn generate(version: &str, ports: Ports) -> Result<Self> {
        let release = crate::napcat::install::catalog::lookup(version)?;
        ports.validate()?;
        let mut random = [0_u8; 96];
        getrandom::fill(&mut random).map_err(|_| {
            AppError::new(
                "RANDOM_UNAVAILABLE",
                "Managed credentials could not be generated.",
                6,
            )
        })?;
        let secrets = random.as_chunks::<32>().0;
        let [api, control, jwt] = std::array::from_fn(|index| {
            secrets[index]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        });
        // An impossible-looking entropy failure still must not collapse trust domains.
        if api == control || api == jwt || control == jwt {
            return Err(AppError::new(
                "RANDOM_UNAVAILABLE",
                "Managed credential domains were not independent.",
                6,
            ));
        }
        Self::restore(release.version, ports, api, control, jwt)
    }

    pub(crate) fn restore(
        version: &str,
        ports: Ports,
        api: String,
        control: String,
        jwt: String,
    ) -> Result<Self> {
        let release = crate::napcat::install::catalog::lookup(version)?;
        ports.validate()?;
        if [&api, &control, &jwt].iter().any(|token| {
            token.len() != 64
                || !token
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) || api == control
            || api == jwt
            || control == jwt
        {
            return Err(AppError::new(
                "RUNTIME_CREDENTIAL_INVALID",
                "Managed credential material is invalid or inconsistent.",
                6,
            ));
        }
        let files = vec![
            file(
                "napcat.json",
                json!({"fileLog":false,"consoleLog":false,"fileLogLevel":"error","consoleLogLevel":"error"}),
            )?,
            file(
                "webui.json",
                json!({"host":"127.0.0.1","port":ports.control,"token":control,"autoLoginAccount":"","disableWebUI":false,"loginRate":10,"accessControlMode":"whitelist","ipWhitelist":["127.0.0.1"],"ipBlacklist":[],"enableXForwardedFor":false,"enable2FA":false,"totpSecret":""}),
            )?,
            file(
                "onebot11.json",
                json!({"network":{
                    "httpServers":[{"name":"qq-cli-http","enable":true,"host":"127.0.0.1","port":ports.http,"token":api,"enableCors":false,"enableWebsocket":false,"messagePostFormat":"array","debug":false}],
                    "websocketServers":[{"name":"qq-cli-events","enable":true,"host":"127.0.0.1","port":ports.events,"token":api,"messagePostFormat":"array","reportSelfMessage":true,"enableForcePushEvent":true,"heartInterval":30000,"debug":false}],
                    "httpSseServers":[],"httpClients":[],"websocketClients":[],"plugins":[]},
                    "enableLocalFile2Url":true,"parseMultMsg":false,"imageDownloadProxy":"","musicSignUrl":"",
                    "timeout":{"baseTimeout":10000,"uploadSpeedKBps":256,"downloadSpeedKBps":256,"maxTimeout":300000}
                }),
            )?,
        ];
        Ok(Self {
            version: release.version,
            ports,
            api,
            control,
            jwt,
            files,
        })
    }
    pub fn version(&self) -> &'static str {
        self.version
    }
    pub fn ports(&self) -> Ports {
        self.ports
    }
    pub fn files(&self) -> impl Iterator<Item = &ConfigFile> {
        self.files.iter()
    }
    pub fn api_token(&self) -> &str {
        &self.api
    }
    pub fn control_token(&self) -> &str {
        &self.control
    }
    pub(crate) fn jwt_secret(&self) -> &str {
        &self.jwt
    }
    /// Add only this generated override after constructing the clean child environment.
    /// Never inherit or supply NAPCAT_WEBUI_SECRET_KEY; upstream logs its value.
    pub fn visit_environment(
        &self,
        mut visitor: impl FnMut(&'static str, &str) -> Result<()>,
    ) -> Result<()> {
        visitor("NAPCAT_WEBUI_JWT_SECRET_KEY", &self.jwt)
    }
}
fn file(name: &'static str, value: Value) -> Result<ConfigFile> {
    let bytes = serde_json::to_vec(&value).map_err(|_| AppError::protocol())?;
    Ok(ConfigFile { name, bytes })
}

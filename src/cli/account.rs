//! Account tasks use ordinary flags; the backend payload is a separate typed protocol.
use super::args::DeadlineArgs;
use crate::core::{
    contracts::scalar::LocalId,
    input::{Text, TextValue},
};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, Args, Serialize)]
pub struct WriteArgs {
    #[arg(long)]
    pub request_id: Option<LocalId>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: DeadlineArgs,
}
#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountCommand {
    /// Set nickname/signature; omitted fields are not deliberately cleared.
    Set {
        #[arg(long)]
        nickname: Option<Text>,
        /// Empty text explicitly clears the signature.
        #[arg(long)]
        signature: Option<TextValue>,
        #[arg(long,requires="nickname",value_parser=["unknown","male","female"])]
        sex: Option<String>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Replace your avatar using an explicit local image (up to 8 MiB).
    Avatar {
        file: PathBuf,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Set QQ presence using native status codes (10 online, 40 invisible, 70 DND).
    Presence {
        #[arg(long)]
        status: u32,
        #[arg(long, default_value_t = 0)]
        ext_status: u32,
        #[arg(long, default_value_t = 0)]
        battery_status: u32,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Set custom presence wording and its icon.
    #[serde(rename = "status-text")]
    StatusText {
        text: TextValue,
        #[arg(long)]
        face_id: u32,
        #[arg(long, default_value_t = 1)]
        face_type: u32,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl AccountCommand {
    pub fn write(&self) -> &WriteArgs {
        match self {
            Self::Set { write, .. }
            | Self::Avatar { write, .. }
            | Self::Presence { write, .. }
            | Self::StatusText { write, .. } => write,
        }
    }
}

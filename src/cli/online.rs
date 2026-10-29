//! Explicit private online transfers use native IDs, separate from message references.
use super::{args::DeadlineArgs, write::WriteArgs};
use crate::core::input::{DecimalId, Limit, NonEmpty, Text};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct SendArgs {
    pub user_id: DecimalId,
    pub path: PathBuf,
    #[arg(long)]
    pub name: Option<NonEmpty>,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Args, Serialize)]
pub struct ElementArgs {
    pub user_id: DecimalId,
    /// Native msgId from online list, not a OneBot message ID or signed reference.
    pub native_msg_id: Text,
    /// Native elementId from online list.
    pub element_id: Text,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnlineCommand {
    /// Send one owned file snapshot to an explicitly selected private user.
    Send(Box<SendArgs>),
    /// Send a bounded owned directory snapshot to an explicitly selected private user.
    SendFolder(Box<SendArgs>),
    /// Read a bounded native online-file batch without automatically receiving files.
    List {
        user_id: DecimalId,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Start receiving an explicit native message element into the SDK default directory.
    Receive(Box<ElementArgs>),
    /// Explicitly refuse one selected native message element.
    Refuse(Box<ElementArgs>),
    /// Cancel sending one selected native online-file message.
    Cancel {
        user_id: DecimalId,
        native_msg_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl OnlineCommand {
    pub fn user(&self) -> &DecimalId {
        match self {
            Self::Send(a) | Self::SendFolder(a) => &a.user_id,
            Self::Receive(a) | Self::Refuse(a) => &a.user_id,
            Self::List { user_id, .. } | Self::Cancel { user_id, .. } => user_id,
        }
    }
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::Send(a) | Self::SendFolder(a) => Some(&a.write),
            Self::Receive(a) | Self::Refuse(a) => Some(&a.write),
            Self::Cancel { write, .. } => Some(write),
            _ => None,
        }
    }
    pub fn timeout(&self) -> u16 {
        self.write()
            .map(|w| w.deadline.timeout.get())
            .unwrap_or_else(|| {
                if let Self::List { deadline, .. } = self {
                    deadline.timeout.get()
                } else {
                    30
                }
            })
    }
}

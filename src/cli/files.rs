//! Filesystem resource IDs are explicit native aliases; only private URL lookup uses f1.
use super::{
    args::{Command, DeadlineArgs},
    write::WriteArgs,
};
use crate::core::{
    domain::Conversation,
    input::{DecimalId, Handle, Limit, Text},
};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct UploadArgs {
    pub conversation: Conversation,
    pub file: PathBuf,
    #[arg(long)]
    pub name: Option<Text>,
    #[arg(long)]
    pub folder: Option<Text>,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileCommand {
    /// Read native group file counts and reported space; some quotas are upstream defaults.
    Info {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read the first bounded group root or folder batch; no full traversal is implied.
    List {
        group_id: DecimalId,
        #[arg(long)]
        folder: Option<Text>,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Create a group root folder.
    Mkdir {
        group_id: DecimalId,
        name: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Explicitly delete a group folder.
    Rmdir {
        group_id: DecimalId,
        folder_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Explicitly delete one listed group file alias.
    Remove {
        group_id: DecimalId,
        file_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Move a group file between explicit native directory IDs.
    Move {
        group_id: DecimalId,
        file_id: Text,
        #[arg(long)]
        from: Text,
        #[arg(long)]
        to: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Rename a group file in its explicit current directory.
    Rename {
        group_id: DecimalId,
        file_id: Text,
        name: Text,
        #[arg(long)]
        folder: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Request native group file transfer; this does not send a cross-chat forward.
    Transfer {
        group_id: DecimalId,
        file_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Upload an explicit local file to a private chat or group directory.
    Upload(Box<UploadArgs>),
    /// Retrieve a group-file URL without following it.
    Url {
        group_id: DecimalId,
        file_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Retrieve a private-file URL only after verifying its signed original attachment.
    PrivateUrl {
        media_ref: Handle,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
}
impl FileCommand {
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::Upload(a) => Some(&a.write),
            Self::Mkdir { write, .. }
            | Self::Rmdir { write, .. }
            | Self::Remove { write, .. }
            | Self::Move { write, .. }
            | Self::Rename { write, .. }
            | Self::Transfer { write, .. } => Some(write),
            _ => None,
        }
    }
    pub fn timeout(&self) -> u16 {
        self.write()
            .map(|w| w.deadline.timeout.get())
            .unwrap_or_else(|| match self {
                Self::Info { deadline, .. }
                | Self::List { deadline, .. }
                | Self::Url { deadline, .. }
                | Self::PrivateUrl { deadline, .. } => deadline.timeout.get(),
                _ => 30,
            })
    }
    pub fn target(&self) -> Option<Conversation> {
        match self {
            Self::Upload(a) => Some(a.conversation.clone()),
            Self::Info { group_id, .. }
            | Self::List { group_id, .. }
            | Self::Mkdir { group_id, .. }
            | Self::Rmdir { group_id, .. }
            | Self::Remove { group_id, .. }
            | Self::Move { group_id, .. }
            | Self::Rename { group_id, .. }
            | Self::Transfer { group_id, .. }
            | Self::Url { group_id, .. } => Some(Conversation::Group(group_id.to_string())),
            _ => None,
        }
    }
}
pub fn reference(command: &Command) -> Option<&Handle> {
    match command {
        Command::File {
            command: FileCommand::PrivateUrl { media_ref, .. },
        } => Some(media_ref),
        _ => None,
    }
}

//! Flash transfers distinguish task acceptance from upload/download completion.
use super::{args::DeadlineArgs, write::WriteArgs};
use crate::core::{
    contracts::flash::Validity,
    domain::Conversation,
    input::{Limit, Text, TextValue},
};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct CreateArgs {
    #[arg(required=true,num_args=1..=32)]
    pub paths: Vec<PathBuf>,
    #[arg(long)]
    pub name: Option<TextValue>,
    #[arg(long, value_enum, default_value = "14")]
    pub validity_days: Validity,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlashCommand {
    /// Create an asynchronous file-set upload from owned file/directory snapshots.
    Create(Box<CreateArgs>),
    /// Read one file-set metadata/status observation without claiming transfer completion.
    Info {
        fileset_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read the upstream's fixed first-depth batch; does not traverse all files.
    List {
        fileset_id: Text,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Resolve an explicitly supplied share code/link through the backend.
    Resolve {
        share_code: Text,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Return the native share link as data without visiting it.
    Link {
        fileset_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Return a native file URL using one explicit name or batch-local index.
    Url {
        fileset_id: Text,
        #[arg(long = "name", conflicts_with = "file_index")]
        file_name: Option<Text>,
        #[arg(long = "index", conflicts_with = "file_name")]
        file_index: Option<u32>,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Send the selected file-set card to one explicit private chat or group.
    Send {
        fileset_id: Text,
        conversation: Conversation,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Start a download task in the backend's default directory; does not publish local files.
    Download {
        fileset_id: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl FlashCommand {
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::Create(a) => Some(&a.write),
            Self::Send { write, .. } | Self::Download { write, .. } => Some(write),
            _ => None,
        }
    }
    pub fn timeout(&self) -> u16 {
        self.write()
            .map(|w| w.deadline.timeout.get())
            .unwrap_or_else(|| match self {
                Self::Info { deadline, .. }
                | Self::List { deadline, .. }
                | Self::Resolve { deadline, .. }
                | Self::Link { deadline, .. }
                | Self::Url { deadline, .. } => deadline.timeout.get(),
                _ => 30,
            })
    }
}

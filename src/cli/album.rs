//! Group album tasks use explicit native resource identifiers, never message references.
use super::{args::DeadlineArgs, write::WriteArgs};
use crate::core::input::{DecimalId, Limit, Text, TextValue};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct UploadArgs {
    pub group_id: DecimalId,
    pub album_id: Text,
    #[arg(long)]
    pub album_name: TextValue,
    #[arg(long, required = true)]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlbumCommand {
    /// Read one native page of album names; completeness is not guaranteed.
    List {
        group_id: DecimalId,
        #[arg(long)]
        attach_info: Option<TextValue>,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read a bounded native media page without granting message or download authority.
    Media {
        group_id: DecimalId,
        album_id: Text,
        #[arg(long)]
        attach_info: Option<TextValue>,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Explicitly remove one media resource by its LLOC.
    Remove {
        group_id: DecimalId,
        album_id: Text,
        lloc: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Set or cancel a media or entire upload-batch like.
    Like {
        group_id: DecimalId,
        album_id: Text,
        batch_id: Text,
        #[arg(long,action=clap::ArgAction::Set)]
        enabled: bool,
        #[arg(long)]
        lloc: Option<Text>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Add an explicit plain-text album comment.
    Comment {
        group_id: DecimalId,
        album_id: Text,
        lloc: Text,
        #[arg(long)]
        content: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Upload one image or a shared batch of up to 32 local images.
    Upload(Box<UploadArgs>),
    /// Upload a local video; the upstream extracts a cover or uses its fallback.
    UploadVideo {
        group_id: DecimalId,
        album_id: Text,
        file: PathBuf,
        #[arg(long)]
        album_name: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl AlbumCommand {
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::List { .. } | Self::Media { .. } => None,
            Self::Upload(args) => Some(&args.write),
            Self::Remove { write, .. }
            | Self::Like { write, .. }
            | Self::Comment { write, .. }
            | Self::UploadVideo { write, .. } => Some(write),
        }
    }
    pub fn timeout(&self) -> u16 {
        self.write()
            .map(|w| w.deadline.timeout.get())
            .unwrap_or_else(|| match self {
                Self::List { deadline, .. } | Self::Media { deadline, .. } => {
                    deadline.timeout.get()
                }
                _ => 30,
            })
    }
    pub fn group(&self) -> &DecimalId {
        match self {
            Self::Upload(args) => &args.group_id,
            Self::List { group_id, .. }
            | Self::Media { group_id, .. }
            | Self::Remove { group_id, .. }
            | Self::Like { group_id, .. }
            | Self::Comment { group_id, .. }
            | Self::UploadVideo { group_id, .. } => group_id,
        }
    }
}

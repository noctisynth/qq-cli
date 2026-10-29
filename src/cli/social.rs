//! Explicit QQ-space publishing; channel compatibility stubs have no domain facade.
use super::write::WriteArgs;
use crate::core::{
    contracts::social::Audience,
    input::{DecimalId, Text, TextValue},
};
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct PostArgs {
    #[arg(long)]
    pub content: TextValue,
    #[arg(long, value_enum)]
    pub audience: Audience,
    #[arg(long)]
    pub user_id: Vec<DecimalId>,
    #[arg(long)]
    pub image: Vec<PathBuf>,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, clap::Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum QzoneCommand {
    /// Publish a post to an explicitly chosen audience, optionally with local images.
    Post(Box<PostArgs>),
    /// Explicitly delete one own-account post by its native TID.
    Delete {
        tid: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl QzoneCommand {
    pub fn write(&self) -> &WriteArgs {
        match self {
            Self::Post(args) => &args.write,
            Self::Delete { write, .. } => write,
        }
    }
}

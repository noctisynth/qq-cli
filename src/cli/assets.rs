//! Explicit own-account favorite and emoji tasks.
use super::{
    args::{Command, DeadlineArgs},
    write::WriteArgs,
};
use crate::core::input::{Limit, Text, TextValue};
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;
#[derive(Debug, clap::Args, Serialize)]
pub struct AddEmojiArgs {
    pub file: PathBuf,
    #[arg(long)]
    pub emoji_id: Option<TextValue>,
    #[arg(long)]
    pub package_id: Option<Text>,
    #[arg(long)]
    pub market: bool,
    #[arg(long,action=clap::ArgAction::Set,default_value="true")]
    pub original: bool,
    #[command(flatten)]
    #[serde(flatten)]
    pub write: WriteArgs,
}
#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FavoriteCommand {
    /// Read the first native batch of favorite summaries; never imply full bodies.
    List {
        #[arg(long, default_value_t = 0)]
        category: u32,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Create an own-account favorite from explicitly supplied opaque UTF-8 data.
    Create {
        #[arg(long)]
        brief: TextValue,
        #[arg(long,conflicts_with_all=["raw_file","stdin"])]
        raw_data: Option<TextValue>,
        #[arg(long, value_name = "FILE", conflicts_with = "stdin")]
        raw_file: Option<PathBuf>,
        #[arg(long)]
        stdin: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmojiCommand {
    /// Import a local file through a retained, content-bound snapshot.
    Add(Box<AddEmojiArgs>),
    /// Read one bounded native batch of favorite emoji metadata; URLs remain data.
    List {
        #[arg(long,default_value_t=Limit::new(48))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Explicitly remove up to 100 emojis using resource IDs, never MD5 aliases.
    Remove {
        #[arg(required=true,num_args=1..=100)]
        res_id: Vec<Text>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Explicitly replace or clear one emoji description using its native identity tuple.
    Describe {
        res_id: Text,
        #[arg(long)]
        emoji_id: Text,
        #[arg(long)]
        md5: TextValue,
        #[arg(long)]
        description: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
pub fn write(command: &Command) -> Option<&WriteArgs> {
    match command {
        Command::File { command } => command.write(),
        Command::Album { command } => command.write(),
        Command::Qzone { command } => Some(command.write()),
        Command::Emoji {
            command: EmojiCommand::Add(args),
        } => Some(&args.write),
        Command::Favorite {
            command: FavoriteCommand::Create { write, .. },
        }
        | Command::Emoji {
            command: EmojiCommand::Remove { write, .. } | EmojiCommand::Describe { write, .. },
        } => Some(write),
        _ => None,
    }
}
pub fn timeout(command: &Command) -> Option<u16> {
    if let Some(write) = write(command) {
        return Some(write.deadline.timeout.get());
    }
    match command {
        Command::File { command } => Some(command.timeout()),
        Command::Album { command } => Some(command.timeout()),
        Command::Favorite {
            command: FavoriteCommand::List { deadline, .. },
        }
        | Command::Emoji {
            command: EmojiCommand::List { deadline, .. },
        } => Some(deadline.timeout.get()),
        _ => None,
    }
}
pub(crate) fn is_mutation(path: &str) -> bool {
    matches!(
        path,
        "file.stage"
            | "file.mkdir"
            | "file.rmdir"
            | "file.remove"
            | "file.move"
            | "file.rename"
            | "file.transfer"
            | "file.upload"
    ) || matches!(
        path,
        "album.remove" | "album.like" | "album.comment" | "album.upload" | "album.upload-video"
    ) || matches!(
        path,
        "qzone.post"
            | "qzone.delete"
            | "favorite.create"
            | "emoji.add"
            | "emoji.remove"
            | "emoji.describe"
    )
}

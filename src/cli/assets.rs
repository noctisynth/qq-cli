//! Explicit own-account favorite and emoji tasks.
use super::{
    args::{Command, DeadlineArgs},
    write::WriteArgs,
};
use crate::core::input::{Limit, Text, TextValue};
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;
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
    matches!(path, "favorite.create" | "emoji.remove" | "emoji.describe")
}

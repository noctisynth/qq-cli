use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "qq-cli",
    version,
    about = "Agent-friendly NapCat CLI (M0: offline command skeleton)",
    long_about = "Offline command skeleton. Business commands return NOT_IMPLEMENTED; no configuration, message input or network is accessed.",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Configuration file (parsed only; loading is not implemented yet).
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check the backend (not implemented; no network access).
    Doctor,
    /// Read backend status (not implemented).
    Status,
    /// Read the current account (not implemented).
    #[command(name = "self")]
    SelfInfo,
    /// Read or send private messages (not implemented).
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },
}

impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Doctor => "doctor",
            Self::Status => "status",
            Self::SelfInfo => "self",
            Self::Message { command } => match command {
                MessageCommand::Get(_) => "message.get",
                MessageCommand::History(_) => "message.history",
                MessageCommand::Send(_) => "message.send",
            },
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Message {
                command: MessageCommand::Send(args),
            } => Some(&args.request_id),
            _ => None,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Read one message by ID (not implemented).
    Get(GetArgs),
    /// Read recent messages from one private conversation (not implemented).
    History(HistoryArgs),
    /// Send one text message (not implemented; never sends in M0).
    Send(SendArgs),
}

#[derive(Debug, Args)]
pub struct GetArgs {
    #[arg(long, value_parser = message_id, allow_hyphen_values = true)]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    #[arg(long, value_parser = user_id)]
    pub user: String,
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=100))]
    pub limit: u16,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    #[arg(long, value_parser = user_id)]
    pub user: String,
    #[arg(long, value_parser = request_id)]
    pub request_id: String,
    #[command(flatten)]
    pub input: TextInput,
}

#[derive(Debug, Args)]
#[group(required = true, multiple = false)]
pub struct TextInput {
    /// Text to send (validated locally; never sent in M0).
    #[arg(long, value_parser = text)]
    pub text: Option<String>,
    /// UTF-8 input file (not opened in M0).
    #[arg(long, value_name = "FILE")]
    pub text_file: Option<PathBuf>,
    /// Read input from stdin (not read in M0).
    #[arg(long)]
    pub stdin: bool,
}

fn user_id(value: &str) -> Result<String, String> {
    if !value.is_empty()
        && !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit())
    {
        Ok(value.to_owned())
    } else {
        Err("Expected a positive decimal identifier".to_owned())
    }
}

fn message_id(value: &str) -> Result<String, String> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(value.to_owned())
    } else {
        Err("Expected a signed decimal identifier".to_owned())
    }
}

fn request_id(value: &str) -> Result<String, String> {
    if (1..=128).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(value.to_owned())
    } else {
        Err("Expected 1 to 128 letters, digits, hyphens or underscores".to_owned())
    }
}

fn text(value: &str) -> Result<String, String> {
    if !value.is_empty() && value.len() <= 64 * 1024 {
        Ok(value.to_owned())
    } else {
        Err("Expected nonempty text of at most 64 KiB".to_owned())
    }
}

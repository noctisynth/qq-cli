use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::domain::{Conversation, decimal_id, request_key};

#[derive(Debug, Parser)]
#[command(
    name = "qq-cli",
    version,
    about = "QQ task interface for coding agents",
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Optional configuration override.
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Select an instance when more than one candidate exists.
    #[arg(long, global = true, value_name = "ID", value_parser = request_key)]
    pub instance: Option<String>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect current identity and readiness without starting QQ.
    Status,
    /// Diagnose environment, compatibility and connection without starting QQ.
    Doctor {
        #[arg(long, value_name = "VERSION")]
        version: Option<String>,
    },
    /// Discover machine-readable command contracts offline.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Discover task instructions and export the agent skill package.
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    /// Manage NapCat installations and QQ/NapCat instances.
    Napcat {
        #[command(subcommand)]
        command: NapcatCommand,
    },
    /// Find friends and read contact information.
    Contact {
        #[command(subcommand)]
        command: ContactCommand,
    },
    /// Find groups and read group information or members.
    Group {
        #[command(subcommand)]
        command: GroupCommand,
    },
    /// Inspect recent conversations; filtering never expands this scope.
    Conversation {
        #[command(subcommand)]
        command: ConversationCommand,
    },
    /// Read, search, export, send or reply to messages.
    Message {
        #[command(subcommand)]
        command: MessageCommand,
    },
    /// Explicitly download attachments or prune owned cache files.
    Media {
        #[command(subcommand)]
        command: MediaCommand,
    },
    /// Watch a bounded event stream.
    Event {
        #[command(subcommand)]
        command: EventCommand,
    },
    /// Inspect persistent operation records without repeating operations.
    Request {
        #[command(subcommand)]
        command: RequestCommand,
    },
}

impl Command {
    /// The same budget covers setup and all subsequent command stages.
    pub fn timeout_seconds(&self) -> u16 {
        match self {
            Self::Contact {
                command: ContactCommand::List(args),
            }
            | Self::Group {
                command: GroupCommand::List(args),
            }
            | Self::Conversation {
                command: ConversationCommand::List(args),
            } => args.page.deadline.timeout,
            Self::Group {
                command: GroupCommand::Members { page, .. },
            } => page.deadline.timeout,
            Self::Message {
                command: MessageCommand::History(args),
            } => args.page.deadline.timeout,
            Self::Message {
                command: MessageCommand::Search(args),
            } => args.history.page.deadline.timeout,
            Self::Message {
                command: MessageCommand::Send(args),
            } => args.content.deadline.timeout,
            Self::Message {
                command: MessageCommand::Reply(args),
            } => args.content.deadline.timeout,
            Self::Message {
                command: MessageCommand::Export(args),
            } => args.deadline.timeout,
            Self::Media {
                command: MediaCommand::Download { deadline, .. },
            }
            | Self::Event {
                command: EventCommand::Watch { deadline, .. },
            } => deadline.timeout,
            Self::Napcat {
                command: NapcatCommand::Start(args),
            } => args.deadline.timeout,
            _ => 30,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    /// List command names and summaries.
    List,
    /// Read one complete command contract.
    Get { command: String },
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// List task instructions.
    List,
    /// Read one task instruction.
    Get { task: String },
    /// Export a skill package without modifying host settings.
    Export {
        #[arg(long, value_name = "DIR")]
        output: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum NapcatCommand {
    /// Prepare a verified complete installation without starting QQ.
    Install {
        #[arg(long, value_name = "VERSION")]
        version: String,
        #[arg(long, value_name = "ARCHIVE")]
        from: Option<PathBuf>,
    },
    /// List local registered installation artifacts and references.
    Installations,
    /// Remove an unreferenced owned installation.
    Uninstall {
        #[arg(value_parser=request_key)]
        installation_id: String,
    },
    /// Prepare and reuse an instance; --new requires a stable operation key.
    Start(StartArgs),
    /// List registered instances, including stopped and login-required ones.
    Instances,
    /// Stop the selected owned instance, retaining its data and installation.
    Stop,
    /// Explicitly restart the selected owned instance.
    Restart,
    /// Unregister a stopped instance without removing QQ or request data.
    Remove {
        #[arg(value_parser = request_key)]
        instance_id: String,
    },
    /// Explicitly change the selected instance's installation version.
    Update {
        #[arg(long, value_name = "VERSION")]
        version: String,
    },
    /// Restore the selected instance's previous installation and migration state.
    Rollback,
}

#[derive(Debug, Args)]
pub struct StartArgs {
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    #[arg(long, requires = "request_id")]
    pub new: bool,
    #[arg(long, requires = "new", value_parser = request_key)]
    pub request_id: Option<String>,
    #[command(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Args)]
pub struct DeadlineArgs {
    /// Total operation deadline in seconds.
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u16).range(1..=300))]
    pub timeout: u16,
}

#[derive(Debug, Args)]
pub struct PageArgs {
    /// Total number of returned records, not backend page size.
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u16).range(1..=1000))]
    pub limit: u16,
    #[arg(long)]
    pub cursor: Option<String>,
    #[command(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long)]
    pub query: Option<String>,
    #[command(flatten)]
    pub page: PageArgs,
}

#[derive(Debug, Subcommand)]
pub enum ContactCommand {
    /// List or filter friends; this scope is independent of recent chats.
    List(ListArgs),
    /// Read one friend's relationship and available user information.
    Get {
        #[arg(value_parser = decimal_id)]
        user_id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum GroupCommand {
    /// List or filter joined groups.
    List(ListArgs),
    /// Read group information.
    Get {
        #[arg(value_parser = decimal_id)]
        group_id: String,
    },
    /// List members of one group.
    Members {
        #[arg(value_parser = decimal_id, required_unless_present = "cursor")]
        group_id: Option<String>,
        #[command(flatten)]
        page: PageArgs,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConversationCommand {
    /// List or filter recent conversations only.
    List(ListArgs),
    /// Read observable metadata for one recent conversation.
    Get { conversation: Conversation },
}

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Read a message, optionally expanding forwarded contents.
    Get {
        message_ref: String,
        #[arg(long)]
        expand: bool,
    },
    /// Read one bounded page of a private or group history.
    History(QueryArgs),
    /// Match text in a bounded history range, reporting scan coverage.
    Search(SearchArgs),
    /// Export messages to a protected resumable file operation.
    Export(ExportArgs),
    /// Send a new message with a caller-provided operation key.
    Send(SendArgs),
    /// Reply to a message, deriving its instance and conversation.
    Reply(ReplyArgs),
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    #[arg(long)]
    pub conversation: Option<Conversation>,
    #[arg(long, value_name = "RFC3339")]
    pub since: Option<String>,
    #[arg(long, value_name = "RFC3339")]
    pub until: Option<String>,
    #[command(flatten)]
    pub page: PageArgs,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub query: Option<String>,
    #[command(flatten)]
    pub history: QueryArgs,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[arg(long)]
    pub conversation: Option<Conversation>,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long, value_name = "RFC3339")]
    pub since: Option<String>,
    #[arg(long, value_name = "RFC3339")]
    pub until: Option<String>,
    #[arg(long, value_name = "FILE")]
    pub output: PathBuf,
    #[arg(long)]
    pub resume: bool,
    #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u16).range(1..=1000))]
    pub limit: u16,
    #[command(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Args)]
pub struct SendArgs {
    #[arg(long)]
    pub conversation: Conversation,
    #[command(flatten)]
    pub content: ContentArgs,
}

#[derive(Debug, Args)]
pub struct ReplyArgs {
    pub message_ref: String,
    #[command(flatten)]
    pub content: ContentArgs,
}

#[derive(Debug, Args)]
pub struct ContentArgs {
    #[arg(long, conflicts_with_all = ["text_file", "stdin"], value_parser = text)]
    pub text: Option<String>,
    #[arg(long, conflicts_with_all = ["text", "stdin"], value_name = "FILE")]
    pub text_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["text", "text_file"])]
    pub stdin: bool,
    #[arg(long, value_name = "FILE")]
    pub image: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    pub file: Vec<PathBuf>,
    #[arg(long, value_parser = decimal_id)]
    pub mention: Vec<String>,
    #[arg(long, value_parser = request_key)]
    pub request_id: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Subcommand)]
pub enum MediaCommand {
    /// Download one referenced attachment without opening or executing it.
    Download {
        media_ref: String,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
        #[command(flatten)]
        deadline: DeadlineArgs,
    },
    /// Remove only unused tool-owned cache files according to retention policy.
    Prune,
}

#[derive(Debug, Subcommand)]
pub enum EventCommand {
    /// Watch a bounded stream with explicit reconnect-gap reporting.
    Watch {
        #[arg(long)]
        conversation: Option<Conversation>,
        #[arg(long, default_value_t = 100, value_parser = clap::value_parser!(u16).range(1..=1000))]
        limit: u16,
        #[command(flatten)]
        deadline: DeadlineArgs,
    },
}

#[derive(Debug, Subcommand)]
pub enum RequestCommand {
    /// Read an operation record; never retry a send or instance creation.
    Get {
        #[arg(value_parser = request_key)]
        request_id: String,
    },
}

fn text(value: &str) -> Result<String, String> {
    if value.is_empty() || value.len() > 64 * 1024 {
        Err("Expected 1 to 65536 UTF-8 bytes".into())
    } else {
        Ok(value.to_owned())
    }
}

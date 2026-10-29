use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

pub use super::contact::ContactCommand;
pub use super::group::GroupCommand;
use crate::core::{
    contracts::scalar::LocalId,
    domain::Conversation,
    input::{DecimalId, Handle, Limit, NonEmpty, Rfc3339, Seconds, StartupSeconds, Text},
};
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
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
    #[arg(long, global = true, value_name = "ID")]
    pub instance: Option<LocalId>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    /// Query and explicitly create own-account favorites.
    Favorite {
        #[command(subcommand)]
        command: super::assets::FavoriteCommand,
    },
    /// Query or explicitly modify own-account favorite emojis.
    Emoji {
        #[command(subcommand)]
        command: super::assets::EmojiCommand,
    },
    /// Explicitly change your QQ account's own profile and presence.
    Account {
        #[command(subcommand)]
        command: super::account::AccountCommand,
    },
    /// Discover backend actions and their reviewed effect boundaries offline.
    Api {
        #[command(subcommand)]
        command: ApiCommand,
    },
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

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiCommand {
    /// Call an action on the selected verified instance; writes/unknown effects require a key.
    Call(ApiCallArgs),
    /// List fixed backend action names; registration does not prove runtime support.
    List {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = Limit::new(50))]
        limit: Limit,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Describe one action; unknown actions retain a conservative write effect.
    Describe {
        action: crate::core::input::ActionName,
    },
}

#[derive(Debug, Args, Serialize)]
pub struct ApiCallArgs {
    pub action: crate::core::input::ActionName,
    #[arg(long, conflicts_with = "stdin", value_name = "FILE")]
    pub params_file: Option<PathBuf>,
    #[arg(long, conflicts_with = "params_file")]
    pub stdin: bool,
    #[arg(long)]
    pub request_id: Option<LocalId>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: DeadlineArgs,
}

impl Command {
    /// The same budget covers setup and all subsequent command stages.
    pub fn timeout_seconds(&self) -> u16 {
        if let Some(timeout) = super::assets::timeout(self) {
            return timeout;
        }
        match self {
            Self::Message { command } if super::message_ops::write(command).is_some() => {
                super::message_ops::write(command).map_or(30, |w| w.deadline.timeout.get())
            }
            Self::Contact { command } if command.timeout().is_some() => {
                command.timeout().unwrap_or(30)
            }
            Self::Group { command } => command.timeout(),
            Self::Account { command } => command.write().deadline.timeout.get(),
            Self::Api {
                command: ApiCommand::Call(args),
            } => args.deadline.timeout.get(),
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            Self::Napcat {
                command: NapcatCommand::Restart,
            } => 900,
            Self::Contact {
                command: ContactCommand::List(args),
            }
            | Self::Conversation {
                command: ConversationCommand::List(args),
            } => args.page.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::History(args),
            } => args.page.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::Search(args),
            } => args.history.page.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::Send(args),
            } => args.content.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::Reply(args),
            } => args.content.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::Export(args),
            } => args.deadline.timeout.get(),
            Self::Message {
                command: MessageCommand::Reactions { deadline, .. },
            }
            | Self::Media {
                command: MediaCommand::Download { deadline, .. },
            }
            | Self::Event {
                command: EventCommand::Watch { deadline, .. },
            } => deadline.timeout.get(),
            Self::Napcat {
                command: NapcatCommand::Start(args),
            } => args.deadline.timeout.get(),
            Self::Napcat {
                command: NapcatCommand::Update { deadline, .. },
            } => deadline.timeout.get(),
            Self::Napcat {
                command: NapcatCommand::Rollback { deadline },
            } => deadline.timeout.get(),
            _ => 30,
        }
    }
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaCommand {
    /// List command names and summaries.
    List,
    /// Read one complete command contract.
    Get { command: String },
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
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
    Uninstall { installation_id: LocalId },
    /// Prepare and reuse an instance; --new requires a stable operation key.
    Start(StartArgs),
    /// List registered instances, including stopped and login-required ones.
    Instances,
    /// End the selected owned QQ process tree and shared window; retain data and installation.
    Stop,
    /// Explicitly restart the selected owned instance.
    Restart,
    /// Unregister a stopped instance without removing QQ or request data.
    Remove { instance_id: LocalId },
    /// Explicitly change the selected instance's installation version.
    Update {
        #[arg(long, value_name = "VERSION")]
        version: String,
        /// Explicitly trial this exact target combination without claiming prior support.
        #[arg(long)]
        verify_compatibility: bool,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: UpdateDeadlineArgs,
    },
    /// Restore the authenticated previous installation in a fresh owned run.
    Rollback {
        #[command(flatten)]
        #[serde(flatten)]
        deadline: UpdateDeadlineArgs,
    },
}

#[derive(Debug, Args, Serialize)]
pub struct StartArgs {
    #[arg(long, value_name = "VERSION")]
    pub version: Option<String>,
    #[arg(long, requires = "request_id")]
    pub new: bool,
    /// Explicitly trial an unverified exact combination in a new instance; does not mark it supported.
    #[arg(long, requires = "new")]
    pub verify_compatibility: bool,
    /// Explicitly ask macOS Keychain to authorize this CLI; approve system dialogs yourself.
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[arg(long)]
    pub authorize_keychain: bool,
    #[arg(long, requires = "new")]
    pub request_id: Option<LocalId>,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: StartDeadlineArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct StartDeadlineArgs {
    /// Total preparation/startup budget, including the first macOS application copy.
    #[arg(long, default_value_t = StartupSeconds::new(if cfg!(all(target_os = "macos", target_arch = "aarch64")) { 900 } else { 30 }))]
    pub timeout: StartupSeconds,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub type UpdateDeadlineArgs = StartDeadlineArgs;
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub type UpdateDeadlineArgs = DeadlineArgs;

#[derive(Debug, Args, Serialize)]
pub struct DeadlineArgs {
    /// Total operation deadline in seconds.
    #[arg(long, default_value_t = Seconds::new(30))]
    pub timeout: Seconds,
}

#[derive(Debug, Args, Serialize)]
pub struct PageArgs {
    /// Total number of returned records, not backend page size.
    #[arg(long, default_value_t = Limit::new(20))]
    pub limit: Limit,
    #[arg(long)]
    pub cursor: Option<Handle>,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct ListArgs {
    #[arg(long)]
    pub query: Option<String>,
    #[command(flatten)]
    #[serde(flatten)]
    pub page: PageArgs,
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationCommand {
    /// List or filter recent conversations only.
    List(ListArgs),
    /// Read observable metadata for one recent conversation.
    Get { conversation: Conversation },
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageCommand {
    /// Explicitly set or remove a verified group message as essence.
    Essence {
        message_ref: Handle,
        #[arg(long,action=clap::ArgAction::Set)]
        enabled: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Change a verified group message's todo state (requires Packet).
    Todo {
        message_ref: Handle,
        #[arg(long, value_enum)]
        state: crate::core::contracts::group_content::TodoState,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Explicitly click a group message button using its verified sequence number.
    Click {
        message_ref: Handle,
        #[arg(long)]
        bot_app_id: crate::core::input::Text,
        #[arg(long)]
        button_id: crate::core::input::TextValue,
        #[arg(long)]
        callback_data: Option<crate::core::input::TextValue>,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Set or cancel an emoji reaction on one signed message.
    React {
        message_ref: Handle,
        #[arg(long)]
        emoji_id: crate::core::input::Text,
        #[arg(long, action=clap::ArgAction::Set)]
        enabled: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Read a bounded reaction list; upstream may return an incomplete snapshot.
    Reactions {
        message_ref: Handle,
        #[arg(long)]
        emoji_id: crate::core::input::Text,
        #[arg(long)]
        emoji_type: Option<crate::core::input::Text>,
        #[arg(long,default_value_t=Limit::new(20))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Explicitly trigger speech recognition for one signed voice message.
    Transcribe {
        message_ref: Handle,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Forward signed messages; multiple messages form a merged forward card.
    Forward {
        #[arg(required=true,num_args=1..=32)]
        message_ref: Vec<Handle>,
        #[arg(long)]
        conversation: Conversation,
        #[arg(long)]
        merged: bool,
        /// Allow possible copies to your own chat when merging different source conversations.
        #[arg(long)]
        allow_self_copy: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Recall a signed message reference; the backend still enforces QQ permissions.
    Recall {
        message_ref: Handle,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Mark an entire conversation read, or explicitly mark all conversations read.
    #[serde(rename = "mark-read")]
    MarkRead {
        #[arg(long,conflicts_with_all=["message_ref","all"])]
        conversation: Option<Conversation>,
        #[arg(long,conflicts_with_all=["conversation","all"])]
        message_ref: Option<Handle>,
        #[arg(long,conflicts_with_all=["conversation","message_ref"])]
        all: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: super::write::WriteArgs,
    },
    /// Read a message, optionally expanding forwarded contents.
    Get {
        message_ref: Handle,
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

#[derive(Debug, Args, Serialize)]
pub struct QueryArgs {
    #[arg(long)]
    pub conversation: Option<Conversation>,
    #[arg(long, value_name = "RFC3339")]
    pub since: Option<Rfc3339>,
    #[arg(long, value_name = "RFC3339")]
    pub until: Option<Rfc3339>,
    #[command(flatten)]
    #[serde(flatten)]
    pub page: PageArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct SearchArgs {
    #[arg(long)]
    pub query: Option<NonEmpty>,
    #[command(flatten)]
    #[serde(flatten)]
    pub history: QueryArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct ExportArgs {
    #[arg(long)]
    pub conversation: Option<Conversation>,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long, value_name = "RFC3339")]
    pub since: Option<Rfc3339>,
    #[arg(long, value_name = "RFC3339")]
    pub until: Option<Rfc3339>,
    #[arg(long, value_name = "FILE")]
    pub output: PathBuf,
    #[arg(long)]
    pub resume: bool,
    #[arg(long, default_value_t = Limit::new(1000))]
    pub limit: Limit,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: DeadlineArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct SendArgs {
    #[arg(long)]
    pub conversation: Conversation,
    #[command(flatten)]
    #[serde(flatten)]
    pub content: ContentArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct ReplyArgs {
    pub message_ref: Handle,
    #[command(flatten)]
    #[serde(flatten)]
    pub content: ContentArgs,
}

#[derive(Debug, Args, Serialize)]
pub struct ContentArgs {
    #[arg(long, conflicts_with_all = ["text_file", "stdin"])]
    pub text: Option<Text>,
    #[arg(long, conflicts_with_all = ["text", "stdin"], value_name = "FILE")]
    pub text_file: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["text", "text_file"])]
    pub stdin: bool,
    #[arg(long, value_name = "FILE")]
    pub image: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    pub file: Vec<PathBuf>,
    #[arg(long)]
    pub mention: Vec<DecimalId>,
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
pub enum MediaCommand {
    /// Download one referenced attachment without opening or executing it.
    Download {
        media_ref: Handle,
        /// Convert record attachments only; original keeps the upstream encoding.
        #[arg(long, value_enum, default_value = "original")]
        format: crate::core::contracts::voice::Format,
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Remove only unused tool-owned cache files according to retention policy.
    Prune,
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCommand {
    /// Watch a bounded stream with explicit reconnect-gap reporting.
    Watch {
        #[arg(long)]
        conversation: Option<Conversation>,
        /// Resume this conversation from a signed event checkpoint (at most 24 hours old).
        #[arg(long, requires = "conversation")]
        resume: Option<Handle>,
        #[arg(long, default_value_t = Limit::new(100))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestCommand {
    /// Read an operation record; never retry a send or instance creation.
    Get { request_id: LocalId },
}

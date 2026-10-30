//! Friend relationships, explicit approvals and bounded profile discovery.
use super::{
    args::{DeadlineArgs, ListArgs},
    write::WriteArgs,
};
use crate::core::input::{DecimalId, Limit, Text, TextValue};
use clap::Subcommand;
use serde::Serialize;
#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContactCommand {
    /// Generate a contact Ark card; this does not send a message.
    Card {
        user_id: DecimalId,
        #[arg(long)]
        phone_number: Option<TextValue>,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Send one explicit private poke (requires Packet).
    Poke {
        user_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Explicitly send a native input-status event code.
    InputStatus {
        user_id: DecimalId,
        #[arg(long)]
        event_type: u32,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Like a user's profile once using an explicit operation key.
    Like {
        user_id: DecimalId,
        #[arg(long,default_value_t=Limit::new(1))]
        times: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Read a user's native presence codes (requires Packet support).
    Status {
        user_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read a bounded profile-like batch; other users require their expected UID.
    Likes {
        #[arg(requires = "expected_uid")]
        user_id: Option<DecimalId>,
        #[arg(long, requires = "user_id")]
        expected_uid: Option<Text>,
        #[arg(long, default_value_t = 0)]
        start: u32,
        #[arg(long,default_value_t=Limit::new(10))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// List or filter friends; independent of recent chats.
    List(ListArgs),
    /// Read an existing friend's available information.
    Get { user_id: DecimalId },
    /// Read a user's profile, including a non-friend.
    Profile {
        user_id: DecimalId,
        #[arg(long)]
        no_cache: bool,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read a bounded snapshot of friend categories and their members.
    Categories {
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read the upstream single batch of one-way friends (at most 99).
    Unilateral {
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read a bounded batch of suspicious friend requests; never approve implicitly.
    Requests {
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// List pending incoming ordinary friend requests; flags feed contact respond.
    FriendRequests {
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Change or explicitly clear a friend's remark.
    Remark {
        user_id: DecimalId,
        remark: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Remove a friend, optionally blocking them or requesting mutual deletion.
    Remove {
        user_id: DecimalId,
        #[arg(long)]
        block: bool,
        #[arg(long)]
        both: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Accept or reject an ordinary friend request using its exact flag.
    Respond {
        flag: Text,
        #[arg(long,action=clap::ArgAction::Set)]
        approve: bool,
        #[arg(long)]
        remark: Option<TextValue>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Accept a suspicious request; the fixed backend has no rejection operation.
    AcceptDoubt {
        flag: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl ContactCommand {
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::Poke { write, .. }
            | Self::InputStatus { write, .. }
            | Self::Like { write, .. }
            | Self::Remark { write, .. }
            | Self::Remove { write, .. }
            | Self::Respond { write, .. }
            | Self::AcceptDoubt { write, .. } => Some(write),
            _ => None,
        }
    }
    pub fn timeout(&self) -> Option<u16> {
        if let Some(w) = self.write() {
            return Some(w.deadline.timeout.get());
        }
        match self {
            Self::Card { deadline, .. }
            | Self::Status { deadline, .. }
            | Self::Likes { deadline, .. }
            | Self::Profile { deadline, .. }
            | Self::Categories { deadline }
            | Self::Unilateral { deadline }
            | Self::Requests { deadline, .. } => Some(deadline.timeout.get()),
            Self::FriendRequests { deadline, .. } => Some(deadline.timeout.get()),
            _ => None,
        }
    }
}
pub(crate) fn is_mutation(path: &str) -> bool {
    matches!(
        path,
        "contact.poke"
            | "contact.input-status"
            | "contact.like"
            | "contact.remark"
            | "contact.remove"
            | "contact.respond"
            | "contact.accept-doubt"
    )
}

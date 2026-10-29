//! Group tasks keep writes explicit and leave the existing read contracts intact.
pub(crate) fn is_mutation(path: &str) -> bool {
    matches!(
        path,
        "group.rename"
            | "group.card"
            | "group.admin"
            | "group.ban"
            | "group.ban-all"
            | "group.kick"
            | "group.kick-many"
            | "group.leave"
            | "group.dismiss"
            | "group.title"
            | "group.avatar"
            | "group.remark"
            | "group.robot-policy"
            | "group.join-policy"
            | "group.search-policy"
            | "group.invite-policy"
            | "group.member-permissions"
            | "group.history-visibility"
    )
}
use super::{
    args::{ListArgs, PageArgs},
    write::WriteArgs,
};
use crate::core::{
    contracts::group::InvitePolicy,
    input::{DecimalId, Text, TextValue},
};
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;
impl clap::ValueEnum for InvitePolicy {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Disabled,
            Self::RequireApproval,
            Self::NoApproval,
            Self::NoApprovalUnder100,
        ]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(match self {
            Self::Disabled => "disabled",
            Self::RequireApproval => "require_approval",
            Self::NoApproval => "no_approval",
            Self::NoApprovalUnder100 => "no_approval_under_100",
        }))
    }
}
#[derive(Debug, Subcommand, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GroupCommand {
    /// List or filter joined groups.
    List(ListArgs),
    /// Read group information.
    Get { group_id: DecimalId },
    /// List a member snapshot; roles may lag changes.
    Members {
        #[arg(required_unless_present = "cursor")]
        group_id: Option<DecimalId>,
        #[command(flatten)]
        #[serde(flatten)]
        page: PageArgs,
    },
    /// Apply the rename group operation once; an accepted receipt is not final effect proof.
    Rename {
        group_id: DecimalId,

        name: Text,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the card group operation once; an accepted receipt is not final effect proof.
    Card {
        group_id: DecimalId,

        user_id: DecimalId,

        card: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the admin group operation once; an accepted receipt is not final effect proof.
    Admin {
        group_id: DecimalId,

        user_id: DecimalId,
        #[arg(long,action=clap::ArgAction::Set)]
        enabled: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Mute a member for explicit seconds; zero removes the mute.
    Ban {
        group_id: DecimalId,

        user_id: DecimalId,
        #[arg(long)]
        duration: u32,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the ban-all group operation once; an accepted receipt is not final effect proof.
    BanAll {
        group_id: DecimalId,
        #[arg(long,action=clap::ArgAction::Set)]
        enabled: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the kick group operation once; an accepted receipt is not final effect proof.
    Kick {
        group_id: DecimalId,

        user_id: DecimalId,
        #[arg(long)]
        reject_add: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the kick-many group operation once; an accepted receipt is not final effect proof.
    KickMany {
        group_id: DecimalId,
        #[arg(long,required=true,num_args=1..)]
        user_id: Vec<DecimalId>,
        #[arg(long)]
        reject_add: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Leave a group without requesting dissolution.
    Leave {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Explicitly dissolve a group you own.
    Dismiss {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Change a special title (requires upstream Packet support).
    Title {
        group_id: DecimalId,

        user_id: DecimalId,

        title: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the avatar group operation once; an accepted receipt is not final effect proof.
    Avatar {
        group_id: DecimalId,

        file: PathBuf,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the remark group operation once; an accepted receipt is not final effect proof.
    Remark {
        group_id: DecimalId,

        remark: TextValue,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the robot-policy group operation once; an accepted receipt is not final effect proof.
    RobotPolicy {
        group_id: DecimalId,
        #[arg(long)]
        member_switch: Option<u32>,
        #[arg(long)]
        member_examine: Option<u32>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the join-policy group operation once; an accepted receipt is not final effect proof.
    JoinPolicy {
        group_id: DecimalId,
        #[arg(long)]
        add_type: u32,
        #[arg(long)]
        question: Option<TextValue>,
        #[arg(long)]
        answer: Option<TextValue>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the search-policy group operation once; an accepted receipt is not final effect proof.
    SearchPolicy {
        group_id: DecimalId,
        #[arg(long)]
        no_code_finger_open: Option<u32>,
        #[arg(long)]
        no_finger_open: Option<u32>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the invite-policy group operation once; an accepted receipt is not final effect proof.
    InvitePolicy {
        group_id: DecimalId,

        policy: InvitePolicy,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the member-permissions group operation once; an accepted receipt is not final effect proof.
    MemberPermissions {
        group_id: DecimalId,
        #[arg(long,action=clap::ArgAction::Set)]
        upload_album: Option<bool>,
        #[arg(long,action=clap::ArgAction::Set)]
        temporary_session: Option<bool>,
        #[arg(long,action=clap::ArgAction::Set)]
        create_group: Option<bool>,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Apply the history-visibility group operation once; an accepted receipt is not final effect proof.
    HistoryVisibility {
        group_id: DecimalId,
        #[arg(long,action=clap::ArgAction::Set)]
        visible: bool,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
}
impl GroupCommand {
    pub fn write(&self) -> Option<&WriteArgs> {
        match self {
            Self::List(_) | Self::Get { .. } | Self::Members { .. } => None,
            Self::Rename { write, .. }
            | Self::Card { write, .. }
            | Self::Admin { write, .. }
            | Self::Ban { write, .. }
            | Self::BanAll { write, .. }
            | Self::Kick { write, .. }
            | Self::KickMany { write, .. }
            | Self::Leave { write, .. }
            | Self::Dismiss { write, .. }
            | Self::Title { write, .. }
            | Self::Avatar { write, .. }
            | Self::Remark { write, .. }
            | Self::RobotPolicy { write, .. }
            | Self::JoinPolicy { write, .. }
            | Self::SearchPolicy { write, .. }
            | Self::InvitePolicy { write, .. }
            | Self::MemberPermissions { write, .. }
            | Self::HistoryVisibility { write, .. } => Some(write),
        }
    }
    pub fn timeout(&self) -> u16 {
        if let Some(w) = self.write() {
            return w.deadline.timeout.get();
        }
        match self {
            Self::List(a) => a.page.deadline.timeout.get(),
            Self::Members { page, .. } => page.deadline.timeout.get(),
            _ => 30,
        }
    }
}

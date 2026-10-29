//! Group tasks keep writes explicit and leave the existing read contracts intact.
pub(crate) fn is_mutation(path: &str) -> bool {
    matches!(
        path,
        "group.rename"
            | "group.respond"
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
    args::{DeadlineArgs, ListArgs, PageArgs},
    write::WriteArgs,
};
use crate::core::{
    contracts::{group::InvitePolicy, group_query::HonorKind},
    input::{DecimalId, Limit, Text, TextValue},
};
use clap::Subcommand;
use serde::Serialize;
use std::path::PathBuf;
impl clap::ValueEnum for HonorKind {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::All,
            Self::Talkative,
            Self::Performer,
            Self::Legend,
            Self::Emotion,
            Self::StrongNewbie,
        ]
    }
    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(match self {
            Self::All => "all",
            Self::Talkative => "talkative",
            Self::Performer => "performer",
            Self::Legend => "legend",
            Self::Emotion => "emotion",
            Self::StrongNewbie => "strong_newbie",
        }))
    }
}
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
    /// Read a bounded group-request batch; source names preserve upstream distinctions.
    Requests {
        #[arg(long)]
        group_id: Option<DecimalId>,
        #[arg(long,default_value="system",value_parser=["system","legacy-notices","doubt"])]
        source: String,
        #[arg(long,default_value_t=Limit::new(50))]
        limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Explicitly accept or reject a group request using its exact string flag.
    Respond {
        flag: Text,
        #[arg(long,action=clap::ArgAction::Set)]
        approve: bool,
        #[arg(long)]
        reason: Option<TextValue>,
        #[arg(long,default_value_t=Limit::new(100))]
        search_limit: Limit,
        #[command(flatten)]
        #[serde(flatten)]
        write: WriteArgs,
    },
    /// Read one member's reported profile and role; default refreshes native information.
    Member {
        group_id: DecimalId,
        user_id: DecimalId,
        #[arg(long)]
        cached: bool,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read detailed group counts and the backend's whole-group mute flag.
    Detail {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Inspect known group owner and extension policy fields.
    Extended {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Retrieve a group share URL without following it.
    Share {
        group_id: DecimalId,
        #[arg(long)]
        full: bool,
        #[arg(long, default_value_t = 73)]
        source_id: u32,
        #[arg(long)]
        additional: Option<TextValue>,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read group honor snapshots; empty data does not establish absence.
    Honors {
        group_id: DecimalId,
        #[arg(long, default_value = "all")]
        kind: HonorKind,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read the backend's muted-member snapshot; empty data may also mean upstream failure.
    Muted {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
    /// Read the reported group/account @all quota.
    MentionQuota {
        group_id: DecimalId,
        #[command(flatten)]
        #[serde(flatten)]
        deadline: DeadlineArgs,
    },
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
            Self::Respond { write, .. }
            | Self::Rename { write, .. }
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
            _ => None,
        }
    }
    pub fn timeout(&self) -> u16 {
        if let Some(w) = self.write() {
            return w.deadline.timeout.get();
        }
        match self {
            Self::Requests { deadline, .. }
            | Self::Member { deadline, .. }
            | Self::Detail { deadline, .. }
            | Self::Extended { deadline, .. }
            | Self::Share { deadline, .. }
            | Self::Honors { deadline, .. }
            | Self::Muted { deadline, .. }
            | Self::MentionQuota { deadline, .. } => deadline.timeout.get(),
            Self::List(a) => a.page.deadline.timeout.get(),
            Self::Members { page, .. } => page.deadline.timeout.get(),
            _ => 30,
        }
    }
}

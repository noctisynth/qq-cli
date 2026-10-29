//! Offline task instructions and a portable, self-contained skill artifact.
use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::core::contracts::discovery::{SkillExport, SkillGuide, SkillName, SkillSummary};

use crate::cli::args::SkillCommand;
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::storage::state::reject_links;

struct Task {
    id: &'static str,
    summary: &'static str,
    commands: &'static [&'static str],
    body: &'static str,
}
const SHARED: &str = include_str!("../../skills/qq-cli/references/shared.md");
const TASKS: &[Task] = &[
    Task {
        id: "contact",
        summary: "Read user profiles and manage friend relationships or explicit approvals",
        commands: &[
            "contact.profile",
            "contact.like",
            "contact.status",
            "contact.likes",
            "contact.poke",
            "contact.card",
            "contact.input-status",
            "contact.categories",
            "contact.unilateral",
            "contact.requests",
            "contact.remark",
            "contact.remove",
            "contact.respond",
            "contact.accept-doubt",
        ],
        body: include_str!("../../skills/qq-cli/references/contact.md"),
    },
    Task {
        id: "group",
        summary: "Manage group identity, members and admission policies",
        commands: &[
            "group.rename",
            "group.requests",
            "group.respond",
            "group.member",
            "group.detail",
            "group.extended",
            "group.share",
            "group.honors",
            "group.muted",
            "group.mention-quota",
            "group.card",
            "group.admin",
            "group.ban",
            "group.ban-all",
            "group.kick",
            "group.kick-many",
            "group.leave",
            "group.dismiss",
            "group.title",
            "group.avatar",
            "group.remark",
            "group.robot-policy",
            "group.join-policy",
            "group.search-policy",
            "group.invite-policy",
            "group.member-permissions",
            "group.history-visibility",
            "group.poke",
            "group.card-json",
            "group.sign",
            "group.signed",
        ],
        body: include_str!("../../skills/qq-cli/references/group.md"),
    },
    Task {
        id: "account",
        summary: "Change your own profile, avatar or QQ presence",
        commands: &[
            "account.set",
            "account.avatar",
            "account.presence",
            "account.status-text",
        ],
        body: include_str!("../../skills/qq-cli/references/account.md"),
    },
    Task {
        id: "discover",
        summary: "Find recipients and resolve instance ambiguity",
        commands: &[
            "contact.list",
            "api.list",
            "api.describe",
            "api.call",
            "contact.get",
            "group.list",
            "group.get",
            "group.members",
            "conversation.list",
            "conversation.get",
            "napcat.instances",
            "status",
        ],
        body: include_str!("../../skills/qq-cli/references/discover.md"),
    },
    Task {
        id: "read",
        summary: "Read, search or export bounded history",
        commands: &[
            "message.get",
            "message.reactions",
            "message.history",
            "message.search",
            "message.export",
        ],
        body: include_str!("../../skills/qq-cli/references/read.md"),
    },
    Task {
        id: "send",
        summary: "Send or reply with durable operation keys",
        commands: &[
            "message.send",
            "message.reply",
            "message.recall",
            "message.react",
            "message.click",
            "message.essence",
            "message.todo",
            "message.forward",
            "message.mark-read",
            "request.get",
        ],
        body: include_str!("../../skills/qq-cli/references/send.md"),
    },
    Task {
        id: "media",
        summary: "Download referenced attachments or prune owned cache",
        commands: &["media.download", "message.transcribe", "media.prune"],
        body: include_str!("../../skills/qq-cli/references/media.md"),
    },
    Task {
        id: "events",
        summary: "Observe a bounded stream and report gaps",
        commands: &["event.watch"],
        body: include_str!("../../skills/qq-cli/references/events.md"),
    },
    Task {
        id: "runtime",
        summary: "Manage fixed installations and independent instances",
        commands: &[
            "doctor",
            "napcat.install",
            "napcat.installations",
            "napcat.uninstall",
            "napcat.start",
            "napcat.instances",
            "napcat.stop",
            "napcat.restart",
            "napcat.remove",
            "napcat.update",
            "napcat.rollback",
            "request.get",
        ],
        body: if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            include_str!("../../skills/qq-cli/references/runtime-macos.md")
        } else {
            include_str!("../../skills/qq-cli/references/runtime.md")
        },
    },
];

pub fn list() -> Vec<SkillSummary> {
    TASKS
        .iter()
        .map(|task| SkillSummary {
            task: task.id,
            summary: task.summary,
        })
        .collect()
}
pub fn get(id: &str) -> Result<SkillGuide> {
    let task = TASKS
        .iter()
        .find(|task| task.id == id)
        .ok_or_else(|| AppError::new("TASK_NOT_FOUND", "Unknown task; use skill list.", 2))?;
    Ok(SkillGuide {
        task: task.id,
        commands: task.commands,
        instructions: format!("{}\n{}", SHARED.trim_end(), task.body),
    })
}

pub fn execute(command: &SkillCommand) -> Result<Envelope> {
    match command {
        SkillCommand::List => Ok(Envelope::typed(list())),
        SkillCommand::Get { task } => Ok(Envelope::typed(get(task)?)),
        SkillCommand::Export { output } => export(output),
    }
}

fn export(output: &Path) -> Result<Envelope> {
    reject_links(output)?;
    let output = std::path::absolute(output).map_err(|_| export_error())?;
    let parent = output
        .parent()
        .ok_or_else(export_error)?
        .canonicalize()
        .map_err(|_| export_error())?;
    reject_links(&parent)?;
    if output.try_exists().map_err(|_| export_error())? {
        return Err(AppError::new(
            "OUTPUT_EXISTS",
            "Skill export target already exists; choose a new directory.",
            2,
        ));
    }
    let temporary = tempfile::Builder::new()
        .prefix(".qq-cli-skill-")
        .tempdir_in(parent)
        .map_err(|_| export_error())?;
    std::fs::create_dir(temporary.path().join("references")).map_err(|_| export_error())?;
    let mut assets = vec![
        (
            "SKILL.md".to_owned(),
            include_str!("../../skills/qq-cli/SKILL.md"),
        ),
        ("references/shared.md".into(), SHARED),
        (
            "references/runtime-macos.md".into(),
            include_str!("../../skills/qq-cli/references/runtime-macos.md"),
        ),
    ];
    assets.extend(
        TASKS
            .iter()
            .map(|task| (format!("references/{}.md", task.id), task.body)),
    );
    for (path, body) in &assets {
        let mut file = File::create_new(temporary.path().join(path)).map_err(|_| export_error())?;
        file.write_all(body.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| export_error())?;
    }
    reject_links(&output)?;
    if output.try_exists().map_err(|_| export_error())? {
        return Err(AppError::new(
            "OUTPUT_EXISTS",
            "Skill export target appeared during preparation; it was preserved.",
            2,
        ));
    }
    // On the supported Windows target, rename publishes the complete directory and refuses an existing target.
    std::fs::rename(temporary.path(), &output).map_err(|_| export_error())?;
    let _ = temporary.keep();
    Ok(Envelope::typed(SkillExport {
        output,
        skill: SkillName::QqCli,
        version: env!("CARGO_PKG_VERSION"),
        files: assets.len(),
    }))
}

fn export_error() -> AppError {
    AppError::new(
        "SKILL_EXPORT_IO",
        "The skill package could not be prepared or published safely.",
        6,
    )
}

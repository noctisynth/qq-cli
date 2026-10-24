//! Freeze a proven descendant closure, then terminate only its saved generations.
//! Matching paths discover candidates; only verified parent links grant membership.
use super::{unknown, unverifiable};
use crate::{
    core::error::Result,
    native::{
        macos_terminate::Target,
        process::{self, ExecutableSet, Pinned},
    },
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

pub(super) const LIMIT: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Fresh,
    Intent,
    Observed,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Member {
    pub target: Target,
    /// Earlier member whose original generation proved this direct parent link.
    pub parent: Option<usize>,
    pub suspend: Phase,
    pub terminate: Phase,
}
impl Member {
    pub fn root(target: Target) -> Self {
        Self {
            target,
            parent: None,
            suspend: Phase::Fresh,
            terminate: Phase::Fresh,
        }
    }
}

/// The runtime implementation checks original locks, registry, j2/d2 and p2.
/// Saving authenticates the previous checkpoint before publishing its successor.
pub(super) trait Checkpoint {
    fn guard(&self) -> Result<()>;
    fn save(&mut self, members: &[Member]) -> Result<()>;
}

pub(super) fn execute(
    files: &ExecutableSet,
    members: &mut Vec<Member>,
    checkpoint: &mut impl Checkpoint,
    deadline: Instant,
) -> Result<()> {
    if members.is_empty() || members.len() > LIMIT {
        return Err(unverifiable());
    }
    // Every recovered member was captured before its original parent disappeared.
    // A reused PID or same-image exec cannot refresh that saved audit generation.
    loop {
        checkpoint.guard()?;
        for index in 0..members.len() {
            suspend(index, members, checkpoint, deadline)?;
        }
        let candidates = census(files, checkpoint, deadline)?;
        let mut added = false;
        let mut foreign = false;
        for candidate in &candidates {
            if members
                .iter()
                .any(|m| m.target.identity() == candidate.identity())
            {
                continue;
            }
            let mut parent = None;
            for (index, member) in members.iter().enumerate() {
                if process::exited(member.target.identity(), deadline)? {
                    continue;
                }
                let original = member.target.reopen(deadline)?;
                if candidate.is_direct_child_of(&original, deadline)? {
                    parent = Some(index);
                    break;
                }
            }
            let Some(parent) = parent else {
                foreign = true;
                continue;
            };
            if members.len() == LIMIT {
                return Err(unverifiable());
            }
            checkpoint.guard()?;
            let target = Target::capture(candidate, deadline)?;
            let original_parent = members[parent].target.reopen(deadline)?;
            if !candidate.is_direct_child_of(&original_parent, deadline)? {
                return Err(unverifiable());
            }
            members.push(Member {
                target,
                parent: Some(parent),
                suspend: Phase::Fresh,
                terminate: Phase::Fresh,
            });
            checkpoint.save(members)?;
            added = true;
        }
        if added {
            continue;
        }
        if foreign {
            return Err(unverifiable());
        }
        for member in members.iter() {
            if process::exited(member.target.identity(), deadline)? {
                continue;
            }
            let original = member.target.reopen(deadline)?;
            if !original.is_suspended(deadline)?
                || !candidates
                    .iter()
                    .any(|p| p.identity() == member.target.identity())
            {
                return Err(unknown());
            }
        }
        break;
    }
    // Parents stay suspended while children exit, so no remaining owned member
    // can fork between the final inventory and termination. Never use killpg.
    for index in (0..members.len()).rev() {
        checkpoint.guard()?;
        if process::exited(members[index].target.identity(), deadline)? {
            continue;
        }
        if members[index].terminate == Phase::Fresh {
            verify_frozen(members, checkpoint, deadline)?;
            let original = members[index].target.reopen(deadline)?;
            members[index].terminate = Phase::Intent;
            checkpoint.save(members)?;
            checkpoint.guard()?;
            members[index].target.terminate_once(&original, deadline)?;
        }
        // Intent on recovery is never resent. Only original-generation exit
        // can acknowledge an interrupted or unknown signal.
        wait(checkpoint, deadline, || {
            process::exited(members[index].target.identity(), deadline)
        })?;
        members[index].terminate = Phase::Observed;
        checkpoint.save(members)?;
    }
    checkpoint.guard()?;
    if !census(files, checkpoint, deadline)?.is_empty() {
        return Err(unverifiable());
    }
    for member in members.iter() {
        if !process::exited(member.target.identity(), deadline)? {
            return Err(unknown());
        }
    }
    checkpoint.guard()
}

fn suspend(
    index: usize,
    members: &mut [Member],
    checkpoint: &mut impl Checkpoint,
    deadline: Instant,
) -> Result<()> {
    checkpoint.guard()?;
    let member = &members[index];
    if process::exited(member.target.identity(), deadline)? {
        return Ok(());
    }
    let original = member.target.reopen(deadline)?;
    if members[index].suspend == Phase::Fresh {
        if !original.is_suspended(deadline)? {
            members[index].suspend = Phase::Intent;
            checkpoint.save(members)?;
            checkpoint.guard()?;
            members[index].target.suspend_once(&original, deadline)?;
        }
    } else if members[index].suspend == Phase::Observed && !original.is_suspended(deadline)? {
        return Err(unknown());
    }
    wait(checkpoint, deadline, || {
        if process::exited(members[index].target.identity(), deadline)? {
            return Ok(true);
        }
        original.is_suspended(deadline)
    })?;
    members[index].suspend = Phase::Observed;
    checkpoint.save(members)
}

fn verify_frozen(
    members: &[Member],
    checkpoint: &impl Checkpoint,
    deadline: Instant,
) -> Result<()> {
    checkpoint.guard()?;
    for member in members {
        if !process::exited(member.target.identity(), deadline)?
            && !member.target.reopen(deadline)?.is_suspended(deadline)?
        {
            return Err(unknown());
        }
    }
    checkpoint.guard()
}

fn census(
    files: &ExecutableSet,
    checkpoint: &impl Checkpoint,
    deadline: Instant,
) -> Result<Vec<Pinned>> {
    for _ in 0..400 {
        checkpoint.guard()?;
        match files.candidates(deadline) {
            Ok(candidates) => {
                checkpoint.guard()?;
                return Ok(candidates);
            }
            Err(error)
                if matches!(
                    error.code,
                    "PROCESS_CENSUS_EPOCH_CHANGED" | "PROCESS_CENSUS_IMAGE_EXITED"
                ) =>
            {
                pause(deadline)?;
            }
            Err(error) => return Err(error),
        }
    }
    Err(unverifiable())
}
fn wait(
    checkpoint: &impl Checkpoint,
    deadline: Instant,
    observation: impl Fn() -> Result<bool>,
) -> Result<()> {
    for _ in 0..1200 {
        checkpoint.guard()?;
        if observation()? {
            checkpoint.guard()?;
            return Ok(());
        }
        pause(deadline)?;
    }
    Err(unknown())
}
fn pause(deadline: Instant) -> Result<()> {
    crate::core::deadline::check(deadline)?;
    std::thread::sleep(
        Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
    );
    crate::core::deadline::check(deadline)
}

//! Shared explicit write controls for typed user tasks.
use super::args::DeadlineArgs;
use crate::core::contracts::scalar::LocalId;
use clap::Args;
use serde::Serialize;
#[derive(Debug, Args, Serialize)]
pub struct WriteArgs {
    #[arg(long)]
    pub request_id: Option<LocalId>,
    #[arg(long)]
    pub dry_run: bool,
    #[command(flatten)]
    #[serde(flatten)]
    pub deadline: DeadlineArgs,
}

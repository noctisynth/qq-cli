use chrono::DateTime;

use crate::cli::args::{Cli, Command, MessageCommand, SkillCommand};
use crate::core::error::{AppError, Result};

impl Cli {
    /// Semantic validation runs before state, input files, credentials or transport are opened.
    pub fn validate(&self) -> Result<()> {
        crate::cli::rules::validate(self)?;
        match &self.command {
            Command::Api {
                command: crate::cli::args::ApiCommand::Call(args),
            } if !args.dry_run
                && args.request_id.is_none()
                && crate::transport::catalog::effect(&args.action)
                    != crate::core::contracts::api::Effect::Read =>
            {
                Err(AppError::new(
                    "REQUEST_ID_REQUIRED",
                    "Writes and unreviewed API actions require --request-id.",
                    2,
                ))
            }
            Command::Napcat {
                command: crate::cli::args::NapcatCommand::Remove { instance_id },
            } if self
                .instance
                .as_ref()
                .is_some_and(|selected| selected != instance_id) =>
            {
                Err(AppError::invalid())
            }
            Command::Media { .. } => Ok(()),
            Command::Message { command } => match command {
                MessageCommand::Send(_) => Ok(()),
                MessageCommand::Reply(_) | MessageCommand::Get { .. } => Ok(()),
                MessageCommand::History(args) => {
                    times(args.since.as_deref(), args.until.as_deref()).map(|_| ())
                }
                MessageCommand::Search(args) => {
                    times(args.history.since.as_deref(), args.history.until.as_deref()).map(|_| ())
                }
                MessageCommand::Export(args) => {
                    times(args.since.as_deref(), args.until.as_deref()).map(|_| ())
                }
            },
            Command::Skill {
                command: SkillCommand::Get { task },
            } if task.is_empty() => Err(AppError::invalid()),
            _ => Ok(()),
        }
    }
}

pub fn times(since: Option<&str>, until: Option<&str>) -> Result<(Option<i64>, Option<i64>)> {
    let parse = |value: &str| {
        DateTime::parse_from_rfc3339(value).map_err(|_| {
            AppError::new(
                "TIME_INVALID",
                "Use RFC3339 times with explicit timezone offsets.",
                2,
            )
        })
    };
    let since = since.map(parse).transpose()?;
    let until = until.map(parse).transpose()?;
    if since.zip(until).is_some_and(|(start, end)| start >= end) {
        return Err(AppError::new(
            "TIME_RANGE_INVALID",
            "The since-inclusive boundary must precede the until-exclusive boundary.",
            2,
        ));
    }
    // QQ message times are integral seconds: ceil both bounds before integer comparison.
    let boundary = |value: DateTime<chrono::FixedOffset>| {
        value
            .timestamp()
            .checked_add(i64::from(value.timestamp_subsec_nanos() != 0))
            .ok_or_else(AppError::invalid)
    };
    Ok((
        since.map(boundary).transpose()?,
        until.map(boundary).transpose()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subsecond_ranges_do_not_include_seconds_before_the_requested_start() {
        let (since, until) = times(
            Some("2026-10-01T00:00:00.100Z"),
            Some("2026-10-01T00:00:00.200Z"),
        )
        .unwrap();
        assert_eq!(
            since, until,
            "valid interval contains no whole QQ message second"
        );
        let (since, until) = times(
            Some("2026-10-01T00:00:00Z"),
            Some("2026-10-01T00:00:00.200Z"),
        )
        .unwrap();
        assert_eq!(until.unwrap() - since.unwrap(), 1);
        assert!(times(Some("2026-10-01"), None).is_err());
    }
}

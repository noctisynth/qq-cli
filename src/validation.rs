use chrono::DateTime;

use crate::cli::{Cli, Command, ContentArgs, MessageCommand, QueryArgs, SkillCommand};
use crate::error::{AppError, Result};

impl Cli {
    /// Semantic validation runs before state, input files, credentials or transport are opened.
    pub fn validate(&self) -> Result<()> {
        match &self.command {
            Command::Media {
                command: crate::cli::MediaCommand::Download { media_ref, .. },
            } => reference(media_ref),
            Command::Message { command } => match command {
                MessageCommand::Send(args) => content(&args.content),
                MessageCommand::Reply(args) => {
                    reference(&args.message_ref)?;
                    content(&args.content)
                }
                MessageCommand::Get { message_ref, .. } => reference(message_ref),
                MessageCommand::History(args) => history(args),
                MessageCommand::Search(args) => {
                    history(&args.history)?;
                    if args.history.page.cursor.is_none()
                        && args.query.as_deref().is_none_or(str::is_empty)
                    {
                        return Err(AppError::invalid());
                    }
                    Ok(())
                }
                MessageCommand::Export(args) => {
                    if !args.resume && args.conversation.is_none() {
                        return Err(AppError::invalid());
                    }
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

fn content(args: &ContentArgs) -> Result<()> {
    if args.image.len().saturating_add(args.file.len()) > 16 {
        return Err(AppError::invalid());
    }
    let has_content = args.text.is_some()
        || args.text_file.is_some()
        || args.stdin
        || !args.image.is_empty()
        || !args.file.is_empty()
        || !args.mention.is_empty();
    if !has_content || (!args.dry_run && args.request_id.is_none()) {
        return Err(AppError::invalid());
    }
    Ok(())
}

fn history(args: &QueryArgs) -> Result<()> {
    if args.conversation.is_none() && args.page.cursor.is_none() {
        return Err(AppError::invalid());
    }
    if let Some(cursor) = &args.page.cursor {
        reference(cursor)?;
    }
    times(args.since.as_deref(), args.until.as_deref()).map(|_| ())
}

fn reference(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 {
        Err(AppError::invalid())
    } else {
        Ok(())
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

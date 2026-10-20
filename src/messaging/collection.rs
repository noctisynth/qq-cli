use std::time::{Duration, Instant};

use crate::core::contracts::resource::{CollectionPage, Coverage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::args::{Command, ContactCommand, ConversationCommand, GroupCommand, PageArgs};
use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::read;
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::transport::client::Backend;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Friends,
    Groups,
    Members(String),
    Recent,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Cursor {
    scope: Scope,
    run_id: String,
    source: Source,
    query: Option<String>,
    offset: usize,
    snapshot: String,
}

pub struct Request<'a> {
    source: Source,
    query: Option<&'a str>,
    page: &'a PageArgs,
}

pub fn request(command: &Command) -> Option<Request<'_>> {
    match command {
        Command::Contact {
            command: ContactCommand::List(args),
        } => Some(Request {
            source: Source::Friends,
            query: args.query.as_deref(),
            page: &args.page,
        }),
        Command::Group {
            command: GroupCommand::List(args),
        } => Some(Request {
            source: Source::Groups,
            query: args.query.as_deref(),
            page: &args.page,
        }),
        Command::Group {
            command: GroupCommand::Members { group_id, page },
        } => Some(Request {
            source: Source::Members(
                group_id
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            ),
            query: None,
            page,
        }),
        Command::Conversation {
            command: ConversationCommand::List(args),
        } => Some(Request {
            source: Source::Recent,
            query: args.query.as_deref(),
            page: &args.page,
        }),
        _ => None,
    }
}

pub fn inferred_instance(command: &Command, references: &References) -> Result<Option<String>> {
    request(command)
        .and_then(|request| request.page.cursor.as_deref())
        .map(|token| {
            references
                .get::<Cursor>("c1", token)
                .map(|cursor| cursor.scope.instance_id)
        })
        .transpose()
}

pub fn has_cursor(command: &Command) -> bool {
    request(command).is_some_and(|request| request.page.cursor.is_some())
}

pub fn execute<B: Backend>(
    request: Request<'_>,
    session: &Session<'_, B>,
    references: &References,
) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(request.page.deadline.timeout));
    execute_until(request, session, references, deadline)
}

pub fn execute_until<B: Backend>(
    request: Request<'_>,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<Envelope> {
    crate::core::deadline::check(deadline)?;
    let cursor = request
        .page
        .cursor
        .as_deref()
        .map(|token| references.get::<Cursor>("c1", token))
        .transpose()?;
    let mut source = request.source;
    let mut query = request.query.map(str::to_owned);
    if let Some(cursor) = &cursor {
        let compatible = source == cursor.source
            || matches!((&source, &cursor.source), (Source::Members(id), Source::Members(_)) if id.is_empty());
        if !compatible
            || query
                .as_ref()
                .is_some_and(|query| Some(query) != cursor.query.as_ref())
        {
            return Err(AppError::new(
                "CURSOR_CONFLICT",
                "Explicit collection arguments conflict with the continuation cursor.",
                2,
            ));
        }
        if cursor.scope.instance_id != session.instance.id
            || cursor.run_id != session.instance.run_id
        {
            return Err(AppError::new(
                "CURSOR_STALE",
                "Collection continuation belongs to another instance or run.",
                4,
            ));
        }
        source = cursor.source.clone();
        query = cursor.query.clone();
    }
    if matches!(&source, Source::Members(id) if id.is_empty()) {
        return Err(AppError::invalid());
    }
    let target = match &source {
        Source::Members(id) => Some(Conversation::Group(id.clone())),
        _ => None,
    };
    let (values, scope) = session.read_in_scope(
        target.as_ref(),
        cursor.as_ref().map(|cursor| &cursor.scope),
        deadline,
        |backend| match &source {
            Source::Friends => read::contacts(backend, deadline),
            Source::Groups => read::groups(backend, deadline),
            Source::Members(id) => read::members(backend, id, deadline),
            Source::Recent => read::conversations(backend, session.config.scan_limit, deadline),
        },
    )?;
    if cursor.as_ref().is_some_and(|cursor| cursor.scope != scope) {
        return Err(AppError::new(
            "ACCOUNT_CHANGED",
            "Continuation identity no longer matches the current account.",
            4,
        ));
    }
    let mut values = read::filter(values, query.as_deref());
    if !session.config.permissions.targets.is_empty() && target.is_none() {
        values.retain(|value| {
            value.conversation().is_some_and(|target| {
                session
                    .config
                    .permissions
                    .targets
                    .iter()
                    .any(|allowed| allowed == target)
            })
        });
    }
    if source != Source::Recent {
        values.sort_by_cached_key(|value| value.id().to_owned());
    }
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&values).map_err(|_| AppError::storage())?)
    );
    let offset = cursor.as_ref().map_or(0, |cursor| cursor.offset);
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.snapshot != digest)
        || offset > values.len()
    {
        return Err(AppError::new(
            "CURSOR_STALE",
            "Collection changed; restart the bounded query.",
            4,
        ));
    }
    let end = offset
        .saturating_add(usize::from(request.page.limit))
        .min(values.len());
    let has_more = end < values.len();
    let next_cursor = if has_more {
        Some(references.issue(
            "c1",
            &Cursor {
                scope: scope.clone(),
                run_id: session.instance.run_id.clone(),
                source: source.clone(),
                query,
                offset: end,
                snapshot: digest,
            },
        )?)
    } else {
        None
    };
    let coverage = match source {
        Source::Friends => Coverage::Friends,
        Source::Groups => Coverage::Groups,
        Source::Members(_) => Coverage::Members,
        Source::Recent => Coverage::Recent,
    };
    Ok(Envelope::collection(
        values[offset..end].to_vec(),
        scope,
        CollectionPage::new(coverage, next_cursor),
    ))
}

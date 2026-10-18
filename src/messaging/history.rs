//! Bounded history traversal. Continuations contain locators, never message bodies.
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cli::args::{Command, MessageCommand, QueryArgs};
use crate::core::domain::{Conversation, Scope};
use crate::core::error::{AppError, Result};
use crate::core::output::Envelope;
use crate::messaging::message::{self, Locator};
use crate::messaging::session::Session;
use crate::storage::references::References;
use crate::transport::client::{Action, Backend};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Cursor {
    scope: Scope,
    run_id: String,
    conversation: Conversation,
    search: bool,
    query: Option<String>,
    since: Option<i64>,
    until: Option<i64>,
    anchor: Option<Locator>,
    pending: Vec<Locator>,
}

pub fn request(command: &Command) -> Option<(&QueryArgs, Option<&str>, bool)> {
    match command {
        Command::Message {
            command: MessageCommand::History(args),
        } => Some((args, None, false)),
        Command::Message {
            command: MessageCommand::Search(args),
        } => Some((&args.history, args.query.as_deref(), true)),
        _ => None,
    }
}

pub fn inferred_instance(command: &Command, references: &References) -> Result<Option<String>> {
    request(command)
        .and_then(|(args, _, _)| args.page.cursor.as_deref())
        .map(|token| {
            references
                .get::<Cursor>("h1", token)
                .map(|cursor| cursor.scope.instance_id)
        })
        .transpose()
}

pub fn execute<B: Backend>(
    args: &QueryArgs,
    query: Option<&str>,
    search: bool,
    session: &Session<'_, B>,
    references: &References,
) -> Result<Envelope> {
    let deadline = Instant::now() + Duration::from_secs(u64::from(args.page.deadline.timeout));
    let page = self::query(args, query, search, session, references, deadline)?;
    Ok(page.envelope())
}

pub struct Page {
    pub messages: Vec<Value>,
    pub scope: Scope,
    pub info: Value,
    pub partial: bool,
}
impl Page {
    pub fn envelope(self) -> Envelope {
        if self.partial {
            Envelope::partial(json!(self.messages), self.info, self.scope)
        } else {
            Envelope::success(json!(self.messages))
                .scoped(self.scope)
                .paged(self.info)
        }
    }
}

pub fn query<B: Backend>(
    args: &QueryArgs,
    query: Option<&str>,
    search: bool,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<Page> {
    let (since, until) =
        crate::cli::validation::times(args.since.as_deref(), args.until.as_deref())?;
    let mut cursor = if let Some(token) = &args.page.cursor {
        let cursor: Cursor = references.get("h1", token)?;
        if cursor.search != search
            || args
                .conversation
                .as_ref()
                .is_some_and(|v| v != &cursor.conversation)
            || query.is_some_and(|q| Some(q) != cursor.query.as_deref())
            || args.since.is_some() && since != cursor.since
            || args.until.is_some() && until != cursor.until
        {
            return Err(AppError::new(
                "CURSOR_CONFLICT",
                "Explicit history arguments conflict with the continuation.",
                2,
            ));
        }
        if cursor.scope.instance_id != session.instance.id
            || cursor.run_id != session.instance.run_id
        {
            return Err(AppError::new(
                "CURSOR_STALE",
                "History continuation belongs to another instance or backend run.",
                4,
            ));
        }
        cursor
    } else {
        let conversation = args.conversation.clone().ok_or_else(AppError::invalid)?;
        session.authorize(Some(&conversation), false)?;
        let info = session.verify(deadline)?;
        Cursor {
            scope: Scope {
                instance_id: session.instance.id.clone(),
                account: info.account,
            },
            run_id: session.instance.run_id.clone(),
            conversation,
            search,
            query: query.map(str::to_owned),
            since,
            until,
            anchor: None,
            pending: vec![],
        }
    };
    session.read_in_scope(
        Some(&cursor.conversation),
        Some(&cursor.scope),
        deadline,
        |_| Ok(()),
    )?;
    let mut output = Vec::new();
    let mut available = HashMap::new();
    let mut scanned = 0_usize;
    let mut pages = 0_usize;
    let mut bytes = 0_usize;
    let mut complete = cursor
        .since
        .zip(cursor.until)
        .is_some_and(|(start, end)| start >= end);
    let mut reason = if complete { "range_satisfied" } else { "limit" };
    let mut interruption: Option<&str> = None;
    while !complete && output.len() < usize::from(args.page.limit) {
        if Instant::now() >= deadline {
            reason = "deadline";
            break;
        }
        if scanned >= session.config.scan_limit {
            reason = "scan_limit";
            break;
        }
        if let Some(locator) = cursor.pending.first().cloned() {
            let message = match available
                .remove(&locator.backend_id)
                .map(Ok)
                .unwrap_or_else(|| message::fetch(&locator, session, references, deadline))
            {
                Ok(message) => message,
                Err(error)
                    if error.code == "ACCOUNT_CHANGED"
                        || error.code == "PERMISSION_DENIED"
                        || error.code == "TARGET_DENIED" =>
                {
                    return Err(error);
                }
                Err(error) => {
                    interruption = Some(error.code);
                    reason = "backend_error";
                    break;
                }
            };
            let size = serde_json::to_vec(&message.value)
                .map_err(|_| AppError::protocol())?
                .len();
            if bytes.saturating_add(size) > session.config.response_bytes {
                reason = "byte_limit";
                break;
            }
            cursor.pending.remove(0);
            scanned += 1;
            let time = message
                .locator
                .time_seconds
                .ok_or_else(AppError::protocol)?;
            if cursor.since.is_none_or(|start| time >= start)
                && cursor.until.is_none_or(|end| time < end)
                && cursor
                    .query
                    .as_ref()
                    .is_none_or(|q| message.text.to_lowercase().contains(&q.to_lowercase()))
            {
                bytes += size;
                output.push(message.value);
            }
            continue;
        }
        if pages >= session.config.page_limit {
            reason = "page_limit";
            break;
        }
        let batch = match load_batch(&cursor, session, references, deadline, &mut pages) {
            Ok(batch) => batch,
            Err(error)
                if error.code == "ACCOUNT_CHANGED"
                    || error.code == "PERMISSION_DENIED"
                    || error.code == "TARGET_DENIED" =>
            {
                return Err(error);
            }
            Err(error) => {
                interruption = Some(error.code);
                reason = match error.code {
                    "HISTORY_PAGE_LIMIT" => "page_limit",
                    "DEADLINE" => "deadline",
                    _ => "backend_error",
                };
                break;
            }
        };
        if batch.is_empty() {
            complete = true;
            reason = "backend_end";
            break;
        }
        let frontier = cursor.anchor.as_ref().map(sequence).transpose()?;
        let mut unique = HashSet::new();
        let mut locators = Vec::new();
        for message in batch {
            let locator = message.locator.clone();
            let seq = sequence(&locator)?;
            if frontier.is_none_or(|old| seq < old) && unique.insert(locator.backend_id.clone()) {
                available.insert(locator.backend_id.clone(), message);
                locators.push(locator);
            }
        }
        locators.sort_by_key(|locator| std::cmp::Reverse(sequence(locator).unwrap_or_default()));
        if locators.is_empty() {
            reason = "no_progress";
            break;
        }
        // A backend sequence must identify one message in this conversation.
        if locators
            .windows(2)
            .any(|pair| pair[0].sequence == pair[1].sequence)
        {
            return Err(AppError::protocol());
        }
        cursor.anchor = locators.last().cloned();
        cursor.pending = locators;
    }
    let next = if complete {
        None
    } else {
        Some(references.issue("h1", &cursor)?)
    };
    let mut page = json!({"complete":complete,"scanned":scanned,"backendPages":pages,"stopReason":reason,
        "coverage":"backend_accessible","order":"newest_first"});
    if let Some(next) = next {
        page["nextCursor"] = json!(next);
    }
    if let Some(code) = interruption {
        page["interruptionCode"] = json!(code);
    }
    if !complete && output.len() < usize::from(args.page.limit) {
        Ok(Page {
            messages: output,
            scope: cursor.scope,
            info: page,
            partial: true,
        })
    } else {
        Ok(Page {
            messages: output,
            scope: cursor.scope,
            info: page,
            partial: false,
        })
    }
}

fn sequence(locator: &Locator) -> Result<u64> {
    locator.sequence.as_ref().and_then(|seq|seq.parse().ok()).ok_or_else(||
        AppError::new("HISTORY_ORDER_UNKNOWN","Backend did not provide a usable conversation sequence; continuation was withheld.",4))
}

fn load_batch<B: Backend>(
    cursor: &Cursor,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
    attempts: &mut usize,
) -> Result<Vec<message::Message>> {
    if let Some(anchor) = &cursor.anchor {
        message::fetch(anchor, session, references, deadline)?;
    }
    let (action, key, id) = match &cursor.conversation {
        Conversation::Private(id) => (Action::FriendHistory, "user_id", id),
        Conversation::Group(id) => (Action::GroupHistory, "group_id", id),
    };
    let mut payload = json!({key:id,"count":20,"reverse_order":true,"disable_get_url":true,"parse_mult_msg":false,"quick_reply":false});
    if let Some(anchor) = &cursor.anchor {
        payload["message_seq"] = json!(anchor.backend_id);
    }
    // Only retry transport failures of this read. The same anchor is retained;
    // count=1 cannot advance an inclusive-anchor backend, so the floor is two.
    let mut counts = [20, 10, 5, 2].into_iter().peekable();
    let raw = loop {
        if Instant::now() >= deadline {
            return Err(AppError::new("DEADLINE", "Operation deadline exceeded.", 3));
        }
        if *attempts >= session.config.page_limit {
            return Err(AppError::new(
                "HISTORY_PAGE_LIMIT",
                "History request budget exhausted.",
                4,
            ));
        }
        payload["count"] = json!(counts.next().ok_or_else(AppError::protocol)?);
        *attempts += 1;
        match session.read_in_scope(
            Some(&cursor.conversation),
            Some(&cursor.scope),
            deadline,
            |backend| backend.call(action, &payload, deadline),
        ) {
            Ok((raw, _)) => break raw,
            Err(error) if error.code == "BACKEND_TRANSPORT" && counts.peek().is_some() => continue,
            Err(error) => return Err(error),
        }
    };
    let messages = raw
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(AppError::protocol)?;
    if messages.len() > 100 {
        return Err(AppError::protocol());
    }
    messages
        .iter()
        .map(|raw| {
            message::normalize(
                raw,
                &cursor.scope,
                &cursor.run_id,
                &cursor.conversation,
                references,
            )
        })
        .collect()
}

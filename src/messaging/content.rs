//! Explicit text input sources; never interpret CQ syntax or infer input from a terminal.
use std::io::{IsTerminal, Read};
use std::time::Instant;

use serde_json::{Value, json};

use crate::cli::args::ContentArgs;
use crate::core::domain::Conversation;
use crate::core::error::{AppError, Result};

pub fn text_from_reader(reader: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(crate::core::input::MAX_TEXT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| input_error())?;
    if bytes.is_empty() || bytes.len() > crate::core::input::MAX_TEXT_BYTES {
        return Err(input_error());
    }
    String::from_utf8(bytes).map_err(|_| input_error())
}

pub fn prepare(args: &ContentArgs, target: &Conversation, deadline: Instant) -> Result<Vec<Value>> {
    if args.image.len().saturating_add(args.file.len()) > crate::core::input::ATTACHMENT_LIMIT {
        return Err(AppError::invalid());
    }
    if args.mention.len() > crate::core::input::MENTION_LIMIT {
        return Err(AppError::invalid());
    }
    if !args.mention.is_empty() && matches!(target, Conversation::Private(_)) {
        return Err(AppError::new(
            "MENTION_UNSUPPORTED",
            "Mentions require a group conversation.",
            2,
        ));
    }
    let text = if let Some(text) = &args.text {
        Some(text.to_string())
    } else if let Some(path) = &args.text_file {
        let file = std::fs::File::open(path).map_err(|_| input_error())?;
        if !file.metadata().map_err(|_| input_error())?.is_file() {
            return Err(input_error());
        }
        Some(text_from_reader(file)?)
    } else if args.stdin {
        if std::io::stdin().is_terminal() {
            return Err(AppError::new(
                "INPUT_REQUIRED",
                "Pipe UTF-8 text into --stdin; interactive prompting is disabled.",
                2,
            ));
        }
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = sender.send(text_from_reader(std::io::stdin()));
        });
        Some(
            receiver
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| {
                    AppError::new(
                        "DEADLINE",
                        "Text input did not finish before the operation deadline.",
                        3,
                    )
                })??,
        )
    } else {
        None
    };
    let mut segments = Vec::new();
    if let Some(text) = text {
        if text.is_empty() || text.len() > crate::core::input::MAX_TEXT_BYTES {
            return Err(input_error());
        }
        segments.push(json!({"type":"text","data":{"text":text}}));
    }
    for id in &args.mention {
        segments.push(json!({"type":"at","data":{"qq":id}}));
    }
    if segments.is_empty() && args.image.is_empty() && args.file.is_empty() {
        return Err(AppError::invalid());
    }
    Ok(segments)
}

fn input_error() -> AppError {
    AppError::new(
        "INPUT_INVALID",
        "Text input must be a readable regular UTF-8 file or stream containing 1 to 65536 bytes.",
        2,
    )
}

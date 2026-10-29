//! One rule list drives both semantic validation and schema projection.
use crate::core::input::{ATTACHMENT_LIMIT, MENTION_LIMIT};
use crate::{
    cli::args::{Cli, Command as CliCommand},
    core::error::{AppError, Result},
};
use clap::{ArgAction, Command, CommandFactory};
use serde_json::{Value, json};
enum Rule {
    Any(&'static [&'static str]),
    Requires(&'static str, &'static [&'static str]),
    Combined(&'static [&'static str], usize),
    Conflict(String, String),
    Max(&'static str, usize),
}
fn active(value: &Value, key: &str) -> bool {
    match &value[key] {
        Value::Null => false,
        Value::Bool(v) => *v,
        Value::Array(v) => !v.is_empty(),
        _ => true,
    }
}
fn property_presence(command: &Command, key: &str) -> Value {
    let mut value = json!({"required":[key]});
    if command.get_arguments().any(|a| {
        a.get_id() == key && matches!(a.get_action(), ArgAction::SetTrue | ArgAction::SetFalse)
    }) {
        value["properties"] = json!({key:{"const":true}});
    } else if command
        .get_arguments()
        .any(|a| a.get_id() == key && matches!(a.get_action(), ArgAction::Append))
    {
        value["properties"] = json!({key:{"minItems":1}});
    }
    value
}
fn rules(path: &str, command: &Command) -> Vec<Rule> {
    let mut rules = match path {
        "account.set" => vec![
            Rule::Any(&["nickname", "signature"]),
            Rule::Any(&["request_id", "dry_run"]),
            Rule::Requires("sex", &["nickname"]),
        ],
        "account.avatar" | "account.presence" | "account.status-text" => {
            vec![Rule::Any(&["request_id", "dry_run"])]
        }
        "message.history" => vec![Rule::Any(&["conversation", "cursor"])],
        "message.search" => vec![
            Rule::Any(&["conversation", "cursor"]),
            Rule::Any(&["query", "cursor"]),
        ],
        "message.export" => vec![Rule::Any(&["conversation", "resume"])],
        "group.members" => vec![Rule::Any(&["group_id", "cursor"])],
        "event.watch" => vec![Rule::Requires("resume", &["conversation"])],
        "napcat.start" => vec![
            Rule::Requires("verify_compatibility", &["new", "request_id"]),
            Rule::Requires("new", &["request_id"]),
            Rule::Requires("request_id", &["new"]),
        ],
        "message.send" | "message.reply" => vec![
            Rule::Any(&["request_id", "dry_run"]),
            Rule::Any(&["text", "text_file", "stdin", "image", "file", "mention"]),
            Rule::Combined(&["image", "file"], ATTACHMENT_LIMIT),
            Rule::Max("mention", MENTION_LIMIT),
        ],
        _ => vec![],
    };
    for arg in command.get_arguments() {
        for conflict in command.get_arg_conflicts_with(arg) {
            if arg.get_id() < conflict.get_id() {
                rules.push(Rule::Conflict(
                    arg.get_id().to_string(),
                    conflict.get_id().to_string(),
                ));
            }
        }
    }
    rules
}
impl Rule {
    fn valid(&self, value: &Value) -> bool {
        match self {
            Self::Any(keys) => keys.iter().any(|k| active(value, k)),
            Self::Requires(key, required) => {
                !active(value, key) || required.iter().all(|k| active(value, k))
            }
            Self::Conflict(a, b) => !active(value, a) || !active(value, b),
            Self::Combined(keys, max) => {
                keys.iter()
                    .map(|k| value[k].as_array().map_or(0, Vec::len))
                    .sum::<usize>()
                    <= *max
            }
            Self::Max(key, max) => value[key].as_array().map_or(0, Vec::len) <= *max,
        }
    }
    fn schemas(&self, command: &Command) -> Vec<Value> {
        match self {
            Self::Any(keys) => vec![
                json!({"anyOf":keys.iter().map(|k|property_presence(command,k)).collect::<Vec<_>>()}),
            ],
            Self::Requires(key, required) => vec![
                json!({"if":property_presence(command,key),"then":{"allOf":required.iter().map(|k|property_presence(command,k)).collect::<Vec<_>>()}}),
            ],
            Self::Conflict(a, b) => vec![
                json!({"not":{"allOf":[property_presence(command,a),property_presence(command,b)]}}),
            ],
            Self::Max(key, max) => vec![json!({"properties":{*key:{"maxItems":max}}})],
            Self::Combined(keys, max) => {
                let mut result = keys
                    .iter()
                    .map(|key| json!({"properties":{*key:{"maxItems":max}}}))
                    .collect::<Vec<_>>();
                if let [left, right] = keys {
                    for count in 1..=*max {
                        result.push(json!({"if":{"required":[left],"properties":{*left:{"minItems":count}}},"then":{"properties":{*right:{"maxItems":max-count}}}}));
                    }
                }
                result
            }
        }
    }
}
pub fn constraints(path: &str, command: &Command) -> Vec<Value> {
    rules(path, command)
        .into_iter()
        .flat_map(|r| r.schemas(command))
        .collect()
}
pub fn validate(cli: &Cli) -> Result<()> {
    let raw = serde_json::to_value(&cli.command).map_err(|_| AppError::invalid())?;
    let (path, mut value) = if let Some(name) = raw.as_str() {
        (name.to_owned(), json!({}))
    } else {
        let (parent, body) = raw
            .as_object()
            .and_then(|v| v.iter().next())
            .ok_or_else(AppError::invalid)?;
        if matches!(cli.command, CliCommand::Status | CliCommand::Doctor { .. }) {
            (parent.clone(), body.clone())
        } else {
            let child = body.get("command").ok_or_else(AppError::invalid)?;
            if let Some(name) = child.as_str() {
                (format!("{parent}.{name}"), json!({}))
            } else {
                let (name, args) = child
                    .as_object()
                    .and_then(|v| v.iter().next())
                    .ok_or_else(AppError::invalid)?;
                (format!("{parent}.{name}"), args.clone())
            }
        }
    };
    if value.is_null() {
        value = json!({});
    }
    if let Some(instance) = &cli.instance {
        value["instance"] = json!(instance);
    }
    let mut root = Cli::command();
    root.build();
    let mut command = &root;
    for part in path.split('.') {
        command = command
            .find_subcommand(part)
            .ok_or_else(AppError::invalid)?;
    }
    if rules(&path, command).iter().any(|r| !r.valid(&value)) {
        return Err(AppError::invalid());
    }
    Ok(())
}

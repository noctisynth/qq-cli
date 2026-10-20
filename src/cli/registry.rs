use clap::{ArgAction, Command, CommandFactory};
use serde_json::{Map, Value, json};

use crate::cli::args::Cli;
use crate::core::error::{AppError, Result};

fn root() -> Command {
    let mut command = Cli::command();
    command.build();
    command
}

pub fn list() -> Value {
    fn visit(command: &Command, prefix: &str, entries: &mut Vec<Value>) {
        for child in command.get_subcommands().filter(|c| c.get_name() != "help") {
            let name = if prefix.is_empty() {
                child.get_name().to_owned()
            } else {
                format!("{prefix}.{}", child.get_name())
            };
            if child.has_subcommands() {
                visit(child, &name, entries);
            } else {
                entries.push(json!({
                    "name": name,
                    "summary": child.get_about().map(ToString::to_string).unwrap_or_default(),
                }));
            }
        }
    }
    let mut entries = Vec::new();
    visit(&root(), "", &mut entries);
    Value::Array(entries)
}

pub fn get(path: &str) -> Result<Value> {
    let command = root();
    let mut selected = &command;
    for part in path.split('.') {
        selected = selected
            .find_subcommand(part)
            .filter(|c| c.get_name() != "help")
            .ok_or_else(|| {
                AppError::new("COMMAND_NOT_FOUND", "Unknown command; use schema list.", 2)
            })?;
    }
    if selected.has_subcommands() || path.is_empty() {
        return Err(AppError::new(
            "COMMAND_NOT_FOUND",
            "Select a leaf command from schema list.",
            2,
        ));
    }
    let mut properties = Map::new();
    let mut required = Vec::new();
    for arg in selected.get_arguments() {
        let id = arg.get_id().as_str();
        if matches!(arg.get_action(), ArgAction::Help | ArgAction::Version) {
            continue;
        }
        let mut property = match arg.get_action() {
            ArgAction::SetTrue | ArgAction::SetFalse => json!({"type": "boolean"}),
            ArgAction::Append => json!({"type": "array", "items": {"type": "string"}}),
            _ if matches!(id, "limit" | "timeout") => json!({"type": "integer", "minimum": 1,
                "maximum": if id == "limit" { 1000 } else { 300 }}),
            _ => json!({"type": "string"}),
        };
        if let Some(help) = arg.get_help() {
            property["description"] = json!(help.to_string());
        }
        if let Some(long) = arg.get_long() {
            property["cliFlag"] = json!(format!("--{long}"));
        } else {
            property["positional"] = json!(true);
        }
        if let Some(default) = arg.get_default_values().first() {
            let default = default.to_string_lossy();
            property["default"] = match property["type"].as_str() {
                Some("integer") => json!(default.parse::<u64>().unwrap_or_default()),
                Some("boolean") => json!(default == "true"),
                _ => json!(default),
            };
        }
        if matches!(
            id,
            "request_id" | "instance" | "installation_id" | "instance_id"
        ) {
            property["pattern"] = json!("^[A-Za-z0-9_-]{1,128}$");
        }
        if id == "conversation" {
            property["pattern"] = json!("^(private|group):[1-9][0-9]{0,31}$");
        }
        if matches!(id, "user_id" | "group_id") {
            property["pattern"] = json!("^[1-9][0-9]{0,31}$");
        }
        if matches!(id, "since" | "until") {
            property["format"] = json!("date-time");
        }
        if id == "text" {
            property["minLength"] = json!(1);
            property["maxUtf8Bytes"] = json!(65536);
        }
        if matches!(id, "image" | "file") {
            property["maxItems"] = json!(16);
        }
        if arg.is_required_set() {
            required.push(json!(id));
        }
        properties.insert(id.to_owned(), property);
    }
    let writes = matches!(
        path,
        "message.send"
            | "message.reply"
            | "napcat.start"
            | "napcat.stop"
            | "napcat.restart"
            | "napcat.install"
            | "napcat.uninstall"
            | "napcat.remove"
            | "napcat.update"
            | "napcat.rollback"
            | "media.download"
            | "media.prune"
            | "skill.export"
            | "message.export"
    );
    let mut input_schema = json!({"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
        "properties":properties,"required":required,"additionalProperties":false});
    let mut constraints = Vec::new();
    if matches!(path, "message.history" | "message.search") {
        constraints.push(json!({"anyOf":[{"required":["conversation"]},{"required":["cursor"]}]}));
        if path == "message.search" {
            constraints.push(json!({"anyOf":[{"required":["query"]},{"required":["cursor"]}]}));
        }
    }
    if path == "message.export" {
        constraints.push(json!({"anyOf":[{"required":["conversation"]},
            {"required":["resume"],"properties":{"resume":{"const":true}}}]}));
    }
    if path == "group.members" {
        constraints.push(json!({"anyOf":[{"required":["group_id"]},{"required":["cursor"]}]}));
    }
    if matches!(path, "message.send" | "message.reply") {
        for images in 1..=16 {
            constraints.push(json!({"if":{"required":["image"],"properties":{"image":{"minItems":images}}},"then":{"properties":{"file":{"maxItems":16-images}}}}));
        }
        constraints.push(json!({"anyOf":[{"required":["request_id"]},
            {"required":["dry_run"],"properties":{"dry_run":{"const":true}}}]}));
        constraints.push(
            json!({"anyOf":[{"required":["text"]},{"required":["text_file"]},
            {"required":["stdin"],"properties":{"stdin":{"const":true}}},
            {"required":["image"],"properties":{"image":{"minItems":1}}},
            {"required":["file"],"properties":{"file":{"minItems":1}}},
            {"required":["mention"],"properties":{"mention":{"minItems":1}}}]}),
        );
        for pair in [
            ["text", "text_file"],
            ["text", "stdin"],
            ["text_file", "stdin"],
        ] {
            let mut exclusive = json!({"required":pair});
            if pair.contains(&"stdin") {
                exclusive["properties"] = json!({"stdin":{"const":true}});
            }
            constraints.push(json!({"not":exclusive}));
        }
    }
    if path == "event.watch" {
        constraints
            .push(json!({"if":{"required":["resume"]},"then":{"required":["conversation"]}}));
    }
    if path == "napcat.start" {
        constraints.push(json!({"if":{"required":["verify_compatibility"],"properties":{"verify_compatibility":{"const":true}}},"then":{"required":["new","request_id"],"properties":{"new":{"const":true}}}}));
        constraints.push(json!({"if":{"required":["new"],"properties":{"new":{"const":true}}},"then":{"required":["request_id"]}}));
        constraints.push(json!({"if":{"required":["request_id"]},"then":{"required":["new"],"properties":{"new":{"const":true}}}}));
    }
    if !constraints.is_empty() {
        input_schema["allOf"] = json!(constraints);
    }
    let mut result = json!({
        "name": path,
        "inputSchema": input_schema,
        "responseSchema": {"type": "object", "required": ["schemaVersion", "ok"],
            "properties": {"schemaVersion": {"const": 1}, "ok": {"type": "boolean"},
                "data": {}, "error": {"type": "object"}, "meta": {"type": "object"}, "pageInfo": {"type": "object"}}},
        "sideEffects": if writes { "explicit operation; inspect task skill" } else { "read" },
        "requestId": if matches!(path,"message.send"|"message.reply") {"required unless dry-run"}
            else if path=="napcat.start" {"required for --new"} else {"not required"},
        "defaults": {"automaticInstanceSelection": "only one candidate", "automaticWriteRetry": false},
    });
    result["outputFormat"] = json!(if path == "event.watch" {
        "ndjson"
    } else {
        "json"
    });
    if let Some(schema) = crate::cli::schema::write_response(path) {
        result["responseSchema"] = schema;
    }
    if path == "media.prune" {
        result["responseSchema"] = crate::cli::schema::prune_response();
    }
    if path == "media.download" {
        result["responseSchema"] = crate::cli::schema::media_response();
    }
    if let Some(schema) = typed_response(path)? {
        result["responseSchema"] = schema;
    }
    if path == "event.watch" {
        result["responseSchema"] = crate::cli::schema::event_stream();
        result["preStreamErrorSchema"] = crate::cli::schema::error_envelope();
        result["streamProtocol"] = json!({"ready":"once after identity checks and connection","preReadyFailure":"single preStreamErrorSchema record","normalTermination":"summary","outputFailure":"no further output; discard incomplete final line"});
    }
    Ok(result)
}

// Only routes: all field definitions belong to the actual serialized contracts.
fn typed_response(path: &str) -> Result<Option<Value>> {
    use crate::core::contracts::{
        installation::{Installation, Installed, RecordedInstance, Uninstalled},
        reply::Reply,
        schema,
    };
    let schema = match path {
        "message.history" | "message.search" => schema::<
            crate::core::contracts::reply::PageReply<
                Vec<crate::core::contracts::message::Message>,
                crate::core::contracts::history::HistoryPage,
            >,
        >(),
        "message.export" => schema::<crate::core::contracts::history::ExportReply>(),
        "message.get" => schema::<Reply<crate::core::contracts::message::Message>>(),
        "contact.get" => schema::<Reply<crate::core::contracts::resource::Contact>>(),
        "group.get" => schema::<Reply<crate::core::contracts::resource::Group>>(),
        "conversation.get" => schema::<Reply<crate::core::contracts::resource::Recent>>(),
        "contact.list" => schema::<
            crate::core::contracts::reply::PagedReply<
                Vec<crate::core::contracts::resource::Contact>,
                crate::core::contracts::resource::CollectionPage,
            >,
        >(),
        "group.list" => schema::<
            crate::core::contracts::reply::PagedReply<
                Vec<crate::core::contracts::resource::Group>,
                crate::core::contracts::resource::CollectionPage,
            >,
        >(),
        "group.members" => schema::<
            crate::core::contracts::reply::PagedReply<
                Vec<crate::core::contracts::resource::Member>,
                crate::core::contracts::resource::CollectionPage,
            >,
        >(),
        "conversation.list" => schema::<
            crate::core::contracts::reply::PagedReply<
                Vec<crate::core::contracts::resource::Recent>,
                crate::core::contracts::resource::CollectionPage,
            >,
        >(),
        "doctor" => schema::<Reply<crate::core::contracts::diagnostic::Diagnostic>>(),
        "status" => schema::<Reply<crate::core::contracts::instance::Status>>(),
        "napcat.start" | "napcat.restart" | "napcat.update" | "napcat.rollback" => {
            schema::<Reply<crate::core::contracts::instance::Started>>()
        }
        "napcat.stop" => schema::<Reply<crate::core::contracts::instance::Stopped>>(),
        "napcat.remove" => schema::<Reply<crate::core::contracts::instance::Removed>>(),
        "napcat.install" => schema::<Reply<Installed>>(),
        "napcat.uninstall" => schema::<Reply<Uninstalled>>(),
        "napcat.installations" => schema::<Reply<Vec<Installation>>>(),
        "napcat.instances" => schema::<Reply<Vec<RecordedInstance>>>(),
        _ => return Ok(None),
    };
    serde_json::to_value(schema)
        .map(Some)
        .map_err(|_| AppError::storage())
}

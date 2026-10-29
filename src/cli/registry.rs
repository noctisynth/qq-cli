use crate::core::contracts::{discovery::*, scalar::No};
use clap::{ArgAction, Command};
use serde_json::{Map, Value, json};

use crate::core::error::{AppError, Result};

fn root() -> Command {
    super::command()
}

pub fn list() -> Vec<CommandSummary> {
    fn visit(command: &Command, prefix: &str, entries: &mut Vec<CommandSummary>) {
        for child in command.get_subcommands().filter(|c| c.get_name() != "help") {
            let name = if prefix.is_empty() {
                child.get_name().to_owned()
            } else {
                format!("{prefix}.{}", child.get_name())
            };
            if child.has_subcommands() {
                visit(child, &name, entries);
            } else {
                entries.push(CommandSummary {
                    name,
                    summary: child
                        .get_about()
                        .map(ToString::to_string)
                        .unwrap_or_default(),
                });
            }
        }
    }
    let mut entries = Vec::new();
    visit(&root(), "", &mut entries);
    entries
}

/// JSON protocol-document view used by offline contract consumers.
pub fn get(path: &str) -> Result<Value> {
    serde_json::to_value(describe(path)?).map_err(|_| AppError::protocol())
}
pub fn describe(path: &str) -> Result<Description> {
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
        let mut property = crate::cli::input_schema::property(arg)?;
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
                Some("integer") => json!(default.parse::<u64>().map_err(|_| AppError::protocol())?),
                Some("boolean") => json!(default == "true"),
                _ => json!(default),
            };
        }
        if arg.is_required_set() {
            required.push(json!(id));
        }
        properties.insert(id.to_owned(), property);
    }
    let writes = path == "api.call"
        || crate::cli::assets::is_mutation(path)
        || crate::cli::group::is_mutation(path)
        || crate::cli::contact::is_mutation(path)
        || matches!(
            path,
            "message.forward"
                | "message.recall"
                | "message.mark-read"
                | "message.transcribe"
                | "message.react"
                | "message.click"
                | "message.essence"
                | "message.todo"
        )
        || path.starts_with("account.")
        || matches!(
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
    let constraints = crate::cli::rules::constraints(path, selected);
    if !constraints.is_empty() {
        input_schema["allOf"] = json!(constraints);
    }
    let response_schema = if path == "event.watch" {
        crate::cli::schema::event_stream()
    } else {
        typed_response(path)?.ok_or_else(AppError::protocol)?
    };
    let stream = path == "event.watch";
    Ok(Description {
        name: path.into(),
        input_schema: SchemaDocument::new(input_schema)?,
        response_schema: SchemaDocument::new(response_schema)?,
        side_effects: if writes {
            "explicit operation; inspect task skill"
        } else {
            "read"
        },
        request_id: if path.starts_with("account.")
            || crate::cli::assets::is_mutation(path)
            || crate::cli::group::is_mutation(path)
            || crate::cli::contact::is_mutation(path)
        {
            "required unless dry-run"
        } else if path == "api.call" {
            "required for writes or unknown effects unless dry-run"
        } else if matches!(
            path,
            "message.forward"
                | "message.send"
                | "message.reply"
                | "message.recall"
                | "message.transcribe"
                | "message.react"
                | "message.click"
                | "message.essence"
                | "message.todo"
                | "message.mark-read"
        ) {
            "required unless dry-run"
        } else if path == "napcat.start" {
            "required for --new"
        } else {
            "not required"
        },
        defaults: Defaults {
            automatic_instance_selection: Selection::OnlyOne,
            automatic_write_retry: No,
        },
        output_format: if stream {
            OutputFormat::Ndjson
        } else {
            OutputFormat::Json
        },
        pre_stream_error_schema: if stream {
            Some(SchemaDocument::new(crate::cli::schema::error_envelope())?)
        } else {
            None
        },
        stream_protocol: if stream {
            Some(StreamProtocol {
                ready: "once after identity checks and connection",
                pre_ready_failure: "single preStreamErrorSchema record",
                normal_termination: "summary",
                output_failure: "no further output; discard incomplete final line",
            })
        } else {
            None
        },
    })
}

// Only routes: all field definitions belong to the actual serialized contracts.
fn typed_response(path: &str) -> Result<Option<Value>> {
    use crate::core::contracts::{
        installation::{Installation, Installed, RecordedInstance, Uninstalled},
        reply::Reply,
        schema,
    };
    let schema = match path {
        "file.info" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::files::Info>,
        >(),
        "file.list" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::files::Listing>,
        >(),
        "file.url" | "file.private-url" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::files::Url>,
        >(),
        "file.mkdir" | "file.rmdir" | "file.remove" | "file.move" | "file.rename"
        | "file.transfer" | "file.upload" => schema::<crate::core::contracts::files::Reply>(),
        "album.list" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::album::Albums>,
        >(),
        "album.media" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::album::Medias>,
        >(),
        "album.remove" | "album.like" | "album.comment" | "album.upload" | "album.upload-video" => {
            schema::<crate::core::contracts::album::Reply>()
        }
        "qzone.post" | "qzone.delete" => schema::<crate::core::contracts::social::Reply>(),
        path if crate::cli::assets::is_mutation(path) => {
            schema::<crate::core::contracts::assets::Reply>()
        }
        "favorite.list" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::assets::Favorites>,
        >(),
        "emoji.list" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::assets::Emojis>,
        >(),
        "contact.card" | "group.card-json" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::card::Card>,
        >(),
        "message.reactions" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::interaction::Reactions,
            >,
        >(),
        "message.transcribe" => schema::<crate::core::contracts::voice::Reply>(),
        "message.essence" | "message.todo" | "message.click" | "message.react"
        | "message.forward" | "message.recall" | "message.mark-read" => {
            schema::<crate::core::contracts::message_ops::Reply>()
        }
        "group.notices" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_content::Notices,
            >,
        >(),
        "group.essence-list" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_content::Essences,
            >,
        >(),
        "group.signed" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_content::Signed,
            >,
        >(),
        "group.respond" => schema::<crate::core::contracts::group_requests::WriteReply>(),
        "group.requests" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_requests::Batch,
            >,
        >(),
        "group.member" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_query::MemberDetails,
            >,
        >(),
        "group.detail" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_query::Details,
            >,
        >(),
        "group.extended" => schema::<
            crate::core::contracts::reply::ScopedReply<
                crate::core::contracts::group_query::Extended,
            >,
        >(),
        "group.share" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::group_query::Share>,
        >(),
        "group.honors" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::group_query::Honors>,
        >(),
        "group.muted" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::group_query::Muted>,
        >(),
        "group.mention-quota" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::group_query::Quota>,
        >(),
        path if crate::cli::contact::is_mutation(path) => {
            schema::<crate::core::contracts::contact::WriteReply>()
        }
        "contact.status" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::UserStatus>,
        >(),
        "contact.likes" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::Likes>,
        >(),
        "contact.profile" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::Profile>,
        >(),
        "contact.categories" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::Categories>,
        >(),
        "contact.unilateral" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::Unilateral>,
        >(),
        "contact.requests" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::contact::Requests>,
        >(),
        path if crate::cli::group::is_mutation(path) => {
            schema::<crate::core::contracts::group::Reply>()
        }
        "schema.list" => schema::<Reply<Vec<CommandSummary>>>(),
        "api.list" => schema::<Reply<crate::core::contracts::api::Catalog>>(),
        "api.describe" => schema::<Reply<crate::core::contracts::api::Description>>(),
        "api.call" => schema::<crate::core::contracts::api::Response>(),
        "account.set" | "account.avatar" | "account.presence" | "account.status-text" => {
            schema::<crate::core::contracts::account::Reply>()
        }
        "schema.get" => schema::<Reply<Description>>(),
        "skill.list" => schema::<Reply<Vec<SkillSummary>>>(),
        "skill.get" => schema::<Reply<SkillGuide>>(),
        "skill.export" => schema::<Reply<SkillExport>>(),
        "message.send" | "message.reply" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::write::WriteResult>,
        >(),
        "request.get" => schema::<crate::core::contracts::write::RequestReply>(),
        "media.download" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::write::Download>,
        >(),
        "media.prune" => schema::<Reply<crate::core::contracts::write::Pruned>>(),
        "message.history" | "message.search" => schema::<
            crate::core::contracts::reply::PageReply<
                Vec<crate::core::contracts::message::Message>,
                crate::core::contracts::history::HistoryPage,
            >,
        >(),
        "message.export" => schema::<crate::core::contracts::history::ExportReply>(),
        "message.get" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::message::Message>,
        >(),
        "contact.get" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::resource::Contact>,
        >(),
        "group.get" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::resource::Group>,
        >(),
        "conversation.get" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::resource::Recent>,
        >(),
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
        "status" => schema::<crate::core::contracts::instance::StatusReply>(),
        "napcat.start" | "napcat.restart" | "napcat.update" | "napcat.rollback" => schema::<
            crate::core::contracts::reply::ScopedReply<crate::core::contracts::instance::Started>,
        >(),
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

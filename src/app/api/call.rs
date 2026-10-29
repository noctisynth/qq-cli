//! One explicit request, shared account/key exclusion, no stored upstream response.
#[cfg(test)]
mod tests;
use crate::{
    cli::args::ApiCallArgs,
    core::{
        contracts::{
            api::{
                Call, Draft, Effect,
                json::{self, Document},
            },
            scalar::{No, Yes},
            write::Outcome,
        },
        domain::{Conversation, Scope},
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    requests::ledger::{self, Begin, Record},
    storage::{references::References, state::Store},
    transport::{catalog, client::Backend},
};
use serde_json::{Value, json};
use std::{fs::File, io::Read, time::Instant};

pub(super) fn parameters(args: &ApiCallArgs) -> Result<Value> {
    let mut bytes = Vec::new();
    if let Some(path) = &args.params_file {
        File::open(path)
            .map_err(|_| input_error())?
            .take(json::MAX_INPUT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| input_error())?;
    } else if args.stdin {
        std::io::stdin()
            .lock()
            .take(json::MAX_INPUT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| input_error())?;
    } else {
        bytes.extend_from_slice(b"{}");
    }
    json::parameters(&bytes)
}
fn input_error() -> AppError {
    AppError::new("API_INPUT", "API parameter input could not be read.", 2)
}
pub(super) fn draft(args: &ApiCallArgs) -> Result<Envelope> {
    let payload = parameters(args)?;
    let description = catalog::describe(&args.action);
    Ok(Envelope::typed(Draft {
        dry_run: Yes,
        action: args.action.to_string(),
        effect: description.effect,
        catalog_known: description.known,
        parameter_bytes: serde_json::to_vec(&payload)
            .map_err(|_| AppError::invalid())?
            .len(),
        identity_verified: No,
    }))
}
fn message_source(value: &Value, escaped: bool) -> bool {
    match value {
        Value::String(text) => {
            !escaped
                && ["[cq:node", "[cq:reply", "[cq:forward"]
                    .iter()
                    .any(|prefix| text.to_ascii_lowercase().contains(prefix))
        }
        Value::Array(items) => items.iter().any(|item| message_source(item, escaped)),
        Value::Object(object) => object
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| matches!(kind, "node" | "reply" | "forward")),
        _ => false,
    }
}
fn reads_message_source(action: &str, payload: &Value) -> bool {
    match catalog::base(action) {
        "set_essence_msg" => true,
        "delete_essence_msg" => !["msg_seq", "msg_random", "group_id"].iter().all(|key| {
            payload
                .get(key)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
        }),
        "set_group_todo" | "complete_group_todo" | "cancel_group_todo" => !payload
            .get("message_seq")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty()),
        "set_msg_emoji_like"
        | "fetch_ptt_text"
        | "fetch_emoji_like"
        | "get_emoji_likes"
        | "click_inline_keyboard_button"
        | ".handle_quick_operation" => true,
        "forward_friend_single_msg"
        | "forward_group_single_msg"
        | "send_forward_msg"
        | "send_group_forward_msg"
        | "send_private_forward_msg" => true,
        "send_msg" | "send_private_msg" | "send_group_msg" => {
            let escaped = payload
                .get("auto_escape")
                .is_some_and(|v| v.as_bool() == Some(true) || v.as_str() == Some("true"));
            payload
                .get("message")
                .is_some_and(|v| message_source(v, escaped))
                || payload
                    .get("messages")
                    .is_some_and(|v| message_source(v, escaped))
        }
        _ => false,
    }
}
fn target(action: &str, payload: &Value, restricted: bool) -> Result<Option<Conversation>> {
    if !restricted {
        return Ok(None);
    }
    let base = catalog::base(action);
    if reads_message_source(action, payload) {
        return Err(AppError::new(
            "API_TARGET_SCOPE_UNKNOWN",
            "Raw message references cannot prove their source targets under the local target restriction.",
            4,
        ));
    }
    let (kind, field) = match base {
        "get_status"
        | "get_login_info"
        | "get_version_info"
        | "set_qq_profile"
        | "set_qq_avatar"
        | "set_self_longnick"
        | "set_online_status"
        | "set_diy_online_status" => return Ok(None),
        "set_group_name"
        | "set_group_card"
        | "set_group_admin"
        | "set_group_ban"
        | "set_group_whole_ban"
        | "set_group_kick"
        | "set_group_kick_members"
        | "set_group_leave"
        | "set_group_special_title"
        | "set_group_portrait"
        | "set_group_remark"
        | "set_group_robot_add_option"
        | "set_group_add_option"
        | "set_group_search"
        | "set_group_member_invite_policy"
        | "set_group_member_permissions"
        | "set_group_new_member_history_visibility"
        | "get_group_info"
        | "get_group_member_list"
        | "get_group_msg_history"
        | "send_group_msg" => ("group", "group_id"),
        "get_group_member_info"
        | "get_group_detail_info"
        | "get_group_info_ex"
        | "get_group_share_link"
        | "get_group_honor_info"
        | "get_group_shut_list"
        | "get_group_at_all_remain" => ("group", "group_id"),
        "mark_msg_as_read" | "mark_private_msg_as_read" | "mark_group_msg_as_read"
            if payload.get("message_id").is_none() =>
        {
            if payload.get("user_id").is_some() {
                ("private", "user_id")
            } else {
                ("group", "group_id")
            }
        }
        "friend_poke" | "group_poke" | "send_poke" => {
            use crate::core::domain::is_exact_js_id;
            let user = payload
                .get("user_id")
                .and_then(Value::as_str)
                .ok_or_else(AppError::invalid)?;
            if !is_exact_js_id(user) {
                return Err(AppError::invalid());
            }
            for field in ["group_id", "target_id"] {
                if payload.get(field).is_some_and(|v| !v.is_string()) {
                    return Err(AppError::invalid());
                }
            }
            if let Some(group) = payload
                .get("group_id")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                if !is_exact_js_id(group) {
                    return Err(AppError::invalid());
                }
                // The target is a member of this same conversation; no private copy is sent.
                if payload
                    .get("target_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .is_some_and(|s| !is_exact_js_id(s))
                {
                    return Err(AppError::invalid());
                }
                ("group", "group_id")
            } else {
                if payload
                    .get("target_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .is_some_and(|target| target != user)
                {
                    return Err(AppError::new(
                        "API_TARGET_SCOPE_UNKNOWN",
                        "A private poke cannot prove a different target under the local policy.",
                        4,
                    ));
                }
                ("private", "user_id")
            }
        }
        "create_collection"
        | "fetch_custom_face"
        | "fetch_custom_face_detail"
        | "delete_custom_face"
        | "set_custom_face_desc"
        | "get_mini_app_ark" => return Ok(None),
        "_send_group_notice"
        | "_get_group_notice"
        | "_del_group_notice"
        | "get_essence_msg_list"
        | "get_group_signed_list"
        | "delete_essence_msg" => ("group", "group_id"),
        "set_group_todo"
        | "complete_group_todo"
        | "cancel_group_todo"
        | "set_group_sign"
        | "send_group_sign" => {
            let id = payload
                .get("group_id")
                .and_then(crate::core::domain::identifier)
                .ok_or_else(AppError::invalid)?;
            if !crate::core::domain::is_exact_js_id(&id) {
                return Err(AppError::invalid());
            }
            ("group", "group_id")
        }
        "ArkSharePeer" | "send_ark_share" => {
            if payload.get("group_id").is_some_and(|v| !v.is_string()) {
                return Err(AppError::invalid());
            }
            if payload
                .get("group_id")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                ("group", "group_id")
            } else {
                ("private", "user_id")
            }
        }
        "ArkShareGroup" | "send_group_ark_share" => ("group", "group_id"),
        "delete_friend" => (
            "private",
            if payload.get("friend_id").is_some_and(|v| !v.is_null()) {
                "friend_id"
            } else {
                "user_id"
            },
        ),
        "get_stranger_info" | "set_friend_remark" | "send_like" | "nc_get_user_status"
        | "set_input_status" => ("private", "user_id"),
        "get_friend_msg_history" | "send_private_msg" if payload.get("group_id").is_none() => {
            ("private", "user_id")
        }
        _ => {
            return Err(AppError::new(
                "API_TARGET_SCOPE_UNKNOWN",
                "This action cannot prove all targets under the configured target restriction.",
                4,
            ));
        }
    };
    let id = payload
        .get(field)
        .and_then(crate::core::domain::identifier)
        .ok_or_else(AppError::invalid)?;
    format!("{kind}:{id}")
        .parse()
        .map(Some)
        .map_err(|_| AppError::invalid())
}
pub(in crate::app) fn execute<B: Backend>(
    args: &ApiCallArgs,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    let payload = parameters(args)?;
    execute_prepared(args, payload, session, store, references, deadline)
}
pub(in crate::app) fn execute_prepared<B: Backend>(
    args: &ApiCallArgs,
    payload: Value,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Envelope> {
    execute_guarded(
        args,
        payload,
        session,
        store,
        references,
        deadline,
        None::<Guard<fn() -> Result<()>>>,
    )
}
/// Private domain evidence, never caller-supplied API parameters or an effect override.
pub(in crate::app) struct Guard<V> {
    pub receipt_target: Option<Conversation>,
    pub targets: Vec<Conversation>,
    pub binding: Value,
    pub validate: V,
}
pub(in crate::app) fn execute_guarded<B: Backend, V: FnOnce() -> Result<()>>(
    args: &ApiCallArgs,
    payload: Value,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
    guard: Option<Guard<V>>,
) -> Result<Envelope> {
    let effect = catalog::effect(&args.action);
    let receipt_target = guard.as_ref().and_then(|g| g.receipt_target.clone());
    if !session.config.permissions.media_download
        && matches!(
            catalog::base(&args.action),
            "get_file"
                | "get_image"
                | "get_record"
                | "download_file"
                | "download_file_stream"
                | "download_file_record_stream"
                | "download_file_image_stream"
                | "get_private_file_url"
                | "get_group_file_url"
                | "get_flash_file_url"
                | "download_fileset"
                | "receive_online_file"
        )
    {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "The local policy does not permit media retrieval.",
            4,
        ));
    }
    let write = effect != Effect::Read;
    if reads_message_source(&args.action, &payload) {
        session.authorize(None, false)?;
    }
    let target = if let Some(guard) = &guard {
        if guard.targets.is_empty() || !write {
            return Err(AppError::invalid());
        }
        for target in &guard.targets {
            session.authorize(Some(target), write)?;
        }
        None
    } else {
        target(
            &args.action,
            &payload,
            !session.config.permissions.targets.is_empty(),
        )?
    };
    session.authorize(target.as_ref(), write)?;
    if !write && args.request_id.is_none() {
        let (raw, scope) = session.read(target.as_ref(), deadline, |backend| {
            backend.call_raw(&args.action, &payload, deadline)
        })?;
        return render(
            args,
            effect,
            (raw, None),
            scope,
            None,
            false,
            session.config.response_bytes,
        );
    }
    let key = args.request_id.as_deref().ok_or_else(|| {
        AppError::new(
            "REQUEST_ID_REQUIRED",
            "Writes and unreviewed API actions require --request-id.",
            2,
        )
    })?;
    let scope = Scope {
        instance_id: session.instance.id.clone(),
        account: match &session.instance.account {
            Some(account) => account.clone(),
            None => session.verify(deadline)?.account,
        },
    };
    let references = references.ok_or_else(AppError::storage)?;
    let operation = format!("api.{}", args.action);
    let mut fingerprint = json!({"action":args.action.to_string(), "parameters":references.api_fingerprint(&payload)?});
    if let Some(guard) = &guard {
        fingerprint["binding"] = json!(references.api_fingerprint(&guard.binding)?);
        if let Some(target) = &receipt_target {
            fingerprint["receiptTarget"] = json!(target.to_string());
        }
    }
    let intent = match ledger::begin(store, key, &operation, &scope, &fingerprint, deadline)? {
        Begin::Existing(record) => return cached(args, effect, record, receipt_target.as_ref()),
        Begin::Fresh(intent) => intent,
    };
    let before = match session.verify(deadline) {
        Ok(before) if before.account == scope.account => before,
        Ok(_) => {
            intent.finish(Outcome::Rejected, None)?;
            let mut error = AppError::new(
                "ACCOUNT_CHANGED",
                "Account changed before the API request; no action was called.",
                4,
            );
            error.outcome = Some(Outcome::Rejected);
            error.request_id = Some(key.into());
            return Err(error.with_scope(scope));
        }
        Err(mut error) => {
            intent.finish(Outcome::Rejected, None)?;
            error.outcome = Some(Outcome::Rejected);
            error.request_id = Some(key.into());
            return Err(error.with_scope(scope));
        }
    };
    if let Some(guard) = guard
        && let Err(mut error) = (guard.validate)()
    {
        intent.finish(Outcome::Rejected, None)?;
        error.outcome = Some(Outcome::Rejected);
        error.request_id = Some(key.into());
        return Err(error.with_scope(scope));
    }
    let raw = match session.backend.call_raw(&args.action, &payload, deadline) {
        Ok(raw) => raw,
        Err(_) => {
            intent.finish(Outcome::Unknown, None)?;
            return Err(ledger::unknown(key).with_scope(scope));
        }
    };
    // Exit/restart can intentionally invalidate the API; the owned socket still guards the response.
    if !matches!(catalog::base(&args.action), "bot_exit" | "set_restart")
        && !session
            .verify(deadline)
            .is_ok_and(|after| after.account == before.account)
    {
        intent.finish(Outcome::Unknown, None)?;
        return Err(ledger::unknown(key).with_scope(scope));
    }
    let outcome = status(&args.action, &raw).map(|(status, code)| {
        if status == "async" || catalog::base(&args.action) == ".handle_quick_operation" {
            Outcome::Unknown
        } else if catalog::base(&args.action) == "fetch_ptt_text" {
            if status == "ok"
                && code == 0
                && raw
                    .get("data")
                    .and_then(|d| d.get("text"))
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
            {
                Outcome::Accepted
            } else {
                Outcome::Unknown
            }
        } else if status == "ok" && code == 0 {
            Outcome::Accepted
        } else {
            Outcome::Rejected
        }
    });
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(_) => {
            intent.finish(Outcome::Unknown, None)?;
            return Err(ledger::unknown(key).with_scope(scope));
        }
    };
    let receipt = if outcome == Outcome::Accepted {
        if let Some(target) = &receipt_target {
            let Some(id) = raw
                .get("data")
                .and_then(|v| v.get("message_id"))
                .and_then(crate::messaging::receipts::message_id)
            else {
                intent.finish(Outcome::Unknown, None)?;
                return Err(ledger::unknown(key).with_scope(scope));
            };
            Some(crate::core::contracts::write::ReceiptIdentity {
                message_id: id,
                conversation: target.to_string(),
            })
        } else {
            None
        }
    } else {
        None
    };
    let stored = receipt
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|_| AppError::storage())?;
    let response = render(
        args,
        effect,
        (raw, receipt),
        scope.clone(),
        Some(outcome.clone()),
        false,
        session.config.response_bytes,
    );
    match response {
        Ok(response) => {
            intent.finish(outcome, stored)?;
            Ok(response)
        }
        Err(_) => {
            intent.finish(Outcome::Unknown, None)?;
            Err(ledger::unknown(key).with_scope(scope))
        }
    }
}
fn status<'a>(action: &str, raw: &'a Value) -> Result<(&'a str, i64)> {
    let (mut status, mut code) = (
        raw.get("status")
            .and_then(Value::as_str)
            .ok_or_else(AppError::protocol)?,
        raw.get("retcode")
            .and_then(Value::as_i64)
            .ok_or_else(AppError::protocol)?,
    );
    if status == "ok"
        && code == 0
        && matches!(
            catalog::base(action),
            "set_qq_profile"
                | "set_self_longnick"
                | "delete_friend"
                | "ArkSharePeer"
                | "send_ark_share"
        )
        && let Some(result) = raw.get("data").and_then(|value| value.get("result"))
    {
        code = result
            .as_i64()
            .or_else(|| result.as_str().and_then(|value| value.parse().ok()))
            .ok_or_else(AppError::protocol)?;
    }
    if status == "ok"
        && code == 0
        && catalog::base(action) == "delete_friend"
        && raw
            .get("data")
            .and_then(|v| v.get("valid"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        status = "failed";
    }
    Ok((status, code))
}
fn render(
    args: &ApiCallArgs,
    effect: Effect,
    raw: (
        Value,
        Option<crate::core::contracts::write::ReceiptIdentity>,
    ),
    scope: Scope,
    outcome: Option<Outcome>,
    deduplicated: bool,
    limit: usize,
) -> Result<Envelope> {
    let (raw, receipt) = raw;
    let (status, code) = status(&args.action, &raw)?;
    let error = if matches!(
        catalog::base(&args.action),
        "fetch_ptt_text" | ".handle_quick_operation"
    ) && outcome == Some(Outcome::Unknown)
    {
        Some(ledger::unknown(
            args.request_id.as_deref().ok_or_else(AppError::protocol)?,
        ))
    } else if status == "async" {
        Some(
            args.request_id
                .as_deref()
                .map(ledger::unknown)
                .unwrap_or_else(|| {
                    AppError::new(
                        "BACKEND_ASYNC",
                        "Backend returned an asynchronous acknowledgement, not a final result.",
                        3,
                    )
                }),
        )
    } else if status != "ok" || code != 0 {
        let mut error = AppError::new(
            "BACKEND_REJECTED",
            "Backend rejected the requested action.",
            4,
        );
        error.outcome = outcome.clone();
        error.request_id = args.request_id.as_deref().map(str::to_owned);
        Some(error)
    } else {
        None
    };
    let response = Envelope::api(
        Call {
            compatibility_only: catalog::compatibility_only(&args.action).then_some(Yes),
            action: args.action.to_string(),
            effect,
            catalog_known: catalog::ACTIONS.contains(&catalog::base(&args.action)),
            deduplicated,
            result_available: true,
            request_id: args.request_id.as_deref().map(str::to_owned),
            outcome,
            result: Some(Document::new(raw)),
            receipt,
        },
        scope,
        error,
    );
    if serde_json::to_vec(&response)
        .map_err(|_| AppError::protocol())?
        .len()
        > limit
    {
        return Err(AppError::new(
            "RESPONSE_LIMIT",
            "The API result exceeds its bounded output size.",
            3,
        ));
    }
    Ok(response)
}
fn cached(
    args: &ApiCallArgs,
    effect: Effect,
    record: Record,
    expected: Option<&Conversation>,
) -> Result<Envelope> {
    let receipt = crate::messaging::receipts::api_message(record.receipt.as_ref(), expected)?;
    if (expected.is_none() && receipt.is_some())
        || (expected.is_some() && record.outcome == Outcome::Accepted && receipt.is_none())
    {
        return Err(AppError::storage());
    }
    let error = match record.outcome {
        Outcome::Accepted => None,
        Outcome::Rejected => Some(AppError::new(
            "BACKEND_REJECTED",
            "This request was previously rejected; no API call was repeated.",
            4,
        )),
        _ => Some(ledger::unknown(&record.request_id)),
    };
    Ok(Envelope::api(
        Call {
            compatibility_only: catalog::compatibility_only(&args.action).then_some(Yes),
            action: args.action.to_string(),
            effect,
            catalog_known: catalog::ACTIONS.contains(&catalog::base(&args.action)),
            receipt,
            request_id: Some(record.request_id),
            outcome: Some(record.outcome),
            deduplicated: true,
            result_available: false,
            result: None,
        },
        record.scope,
        error,
    ))
}

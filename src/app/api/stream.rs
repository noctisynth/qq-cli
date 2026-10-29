//! One raw stream call; partial packets are provisional until the summary.
use crate::{
    cli::{args::ApiCallArgs, stream_output::Output},
    core::{
        contracts::{
            api::{Effect, json::Document, stream::Event},
            scalar::Yes,
            write::Outcome,
        },
        domain::Scope,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::session::Session,
    requests::ledger::{self, Begin},
    storage::{references::References, state::Store},
    transport::{catalog, client::Backend},
};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
fn emit(
    output: &mut Output,
    event: Event,
    scope: &Scope,
    deadline: Instant,
    summary: bool,
) -> Result<()> {
    output.api_record(
        &Envelope::typed(event).scoped(scope.clone()),
        deadline,
        summary,
    )
}
fn exit(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Accepted => 0,
        Outcome::Rejected => 4,
        _ => 5,
    }
}
pub(in crate::app) fn execute<B: Backend>(
    args: &ApiCallArgs,
    payload: Value,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
    sinks: (&mut Output, &AtomicBool),
) -> Result<u8> {
    let (output, cancel) = sinks;
    let effect = catalog::effect(&args.action);
    let write = effect != Effect::Read;
    if super::call::reads_message_source(&args.action, &payload)
        || matches!(
            catalog::base(&args.action),
            "download_fileset" | "receive_online_file"
        )
    {
        session.authorize(None, false)?;
    }
    if !session.config.permissions.media_download && super::call::requires_media(&args.action) {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "The local policy does not permit media retrieval.",
            4,
        ));
    }
    let target = super::call::target(
        &args.action,
        &payload,
        !session.config.permissions.targets.is_empty(),
    )?;
    session.authorize(target.as_ref(), write)?;
    let scope = Scope {
        instance_id: session.instance.id.clone(),
        account: session
            .instance
            .account
            .clone()
            .ok_or_else(AppError::protocol)?,
    };
    let intent = if let Some(key) = args.request_id.as_deref() {
        let references = references.ok_or_else(AppError::storage)?;
        let fingerprint = json!({"action":args.action.to_string(),"parameters":references.api_fingerprint(&payload)?});
        match ledger::begin(
            store,
            key,
            &format!("api.{}", args.action),
            &scope,
            &fingerprint,
            deadline,
        )? {
            Begin::Existing(record) => {
                if record.receipt.is_some() {
                    return Err(AppError::storage());
                }
                let code = exit(&record.outcome);
                emit(
                    output,
                    Event::Summary {
                        action: args.action.to_string(),
                        outcome: record.outcome,
                        deduplicated: true,
                        records: 0,
                        packets_available: false,
                        request_id: Some(key.into()),
                        transfer_complete: None,
                        error_code: None,
                    },
                    &record.scope,
                    deadline,
                    true,
                )?;
                return Ok(code);
            }
            Begin::Fresh(intent) => Some(intent),
        }
    } else {
        if write {
            return Err(AppError::new(
                "REQUEST_ID_REQUIRED",
                "Native writes and unknown streams require a request key.",
                2,
            ));
        }
        None
    };
    if let Err(error) = session.verify(deadline) {
        if let Some(intent) = intent {
            intent.finish(Outcome::Rejected, None)?;
        }
        return Err(error.with_scope(scope));
    }
    if let Err(error) = emit(
        output,
        Event::Ready {
            action: args.action.to_string(),
            effect,
            identity_verified: Yes,
        },
        &scope,
        deadline,
        false,
    ) {
        if let Some(intent) = intent {
            intent.finish(Outcome::Rejected, None)?;
        }
        return Err(error);
    }
    let mut count = 0_usize;
    let mut final_packet = None;
    let mut rejected = false;
    let mut asynchronous = false;
    let mut finished = false;
    let streamed = session
        .backend
        .call_stream(&args.action, &payload, deadline, &mut |raw| {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::new(
                    "CANCELLED",
                    "Native stream was cancelled.",
                    130,
                ));
            }
            if finished {
                return Err(AppError::protocol());
            }
            count += 1;
            let status = raw.get("status").and_then(Value::as_str);
            let code = raw.get("retcode").and_then(Value::as_i64);
            if status == Some("async") {
                asynchronous = true;
                finished = true;
            } else if status != Some("ok") || code != Some(0) {
                rejected = true;
                finished = true;
            }
            if raw.pointer("/data/type").and_then(Value::as_str) == Some("response") {
                finished = true;
            }
            final_packet = Some(raw.get("data").cloned().unwrap_or(Value::Null));
            emit(
                output,
                Event::Packet {
                    action: args.action.to_string(),
                    index: count - 1,
                    provisional: Yes,
                    result: Document::new(raw),
                },
                &scope,
                deadline,
                false,
            )
        });
    let identity = if streamed.is_ok() {
        session.verify(deadline).map(|_| ())
    } else {
        Ok(())
    };
    let transfer_complete = final_packet.as_ref().is_some_and(|v| {
        v.get("type").and_then(Value::as_str) == Some("response")
            && (v.get("data_type").and_then(Value::as_str) == Some("file_complete")
                || v.get("status").and_then(Value::as_str) == Some("file_complete"))
    });
    let is_upload = &*args.action == "upload_file_stream";
    let upload_ack = is_upload
        && count == 1
        && final_packet.as_ref().is_some_and(|v| {
            let same_id = v
                .get("stream_id")
                .and_then(Value::as_str)
                .zip(payload.get("stream_id").and_then(Value::as_str))
                .is_some_and(|(a, b)| a == b);
            same_id
                && matches!(
                    (
                        v.get("type").and_then(Value::as_str),
                        v.get("status").and_then(Value::as_str)
                    ),
                    (Some("stream"), Some("file_created" | "chunk_received"))
                        | (Some("response"), Some("file_complete"))
                )
        });
    let (outcome, error_code) = if let Err(error) = &streamed {
        (Outcome::Unknown, Some(error.code))
    } else if let Err(error) = identity {
        (Outcome::Unknown, Some(error.code))
    } else if asynchronous {
        (Outcome::Unknown, Some("BACKEND_ASYNC"))
    } else if rejected {
        (Outcome::Rejected, Some("BACKEND_REJECTED"))
    } else if if is_upload { upload_ack } else { finished } {
        (Outcome::Accepted, None)
    } else {
        (Outcome::Unknown, Some("STREAM_UNCONFIRMED"))
    };
    let code = exit(&outcome);
    // Write the terminal result before the summary; losing stdout cannot cause a new native call.
    if let Some(intent) = intent {
        intent.finish(outcome.clone(), None)?;
    }
    let summary_deadline = deadline.max(Instant::now() + Duration::from_secs(1));
    emit(
        output,
        Event::Summary {
            action: args.action.to_string(),
            outcome,
            deduplicated: false,
            records: count,
            packets_available: count > 0,
            request_id: args.request_id.as_ref().map(ToString::to_string),
            transfer_complete: Some(code == 0 && transfer_complete),
            error_code,
        },
        &scope,
        summary_deadline,
        true,
    )?;
    Ok(code)
}

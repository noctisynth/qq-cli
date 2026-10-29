//! Explicit backend-cache upload is one durable intent across creation, chunks and completion.
use crate::{
    cli::files::FileCommand,
    core::{
        contracts::{
            files::{StageDraft, StageOperation, StageReceipt},
            scalar::{LocalId, No, Yes},
            streaming::UploadPayload,
            write::Outcome,
        },
        domain::Scope,
        error::{AppError, Result},
        output::Envelope,
    },
    messaging::{attachments, session::Session},
    requests::ledger::{self, Begin},
    storage::{references::References, state::Store},
    transport::client::Backend,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{io::Read, time::Instant};
const CHUNK: usize = 65536;
fn payload(id: &str) -> UploadPayload {
    UploadPayload {
        stream_id: id.into(),
        chunk_data: None,
        chunk_index: None,
        total_chunks: None,
        file_size: None,
        expected_sha256: None,
        is_complete: None,
        filename: None,
        reset: None,
        verify_only: None,
        file_retention: 300000.0,
    }
}
fn call<B: Backend>(
    session: &Session<'_, B>,
    payload: &UploadPayload,
    deadline: Instant,
) -> Result<Value> {
    let payload = serde_json::to_value(payload).map_err(|_| AppError::invalid())?;
    let mut data = None;
    session.backend.call_stream(
        &"upload_file_stream"
            .parse()
            .map_err(|_| AppError::protocol())?,
        &payload,
        deadline,
        &mut |packet| {
            if data.is_some()
                || packet.get("status").and_then(Value::as_str) != Some("ok")
                || packet.get("retcode").and_then(Value::as_i64) != Some(0)
            {
                return Err(AppError::new(
                    "STREAM_UPLOAD_UNCONFIRMED",
                    "Native upload did not provide an unambiguous acknowledgement.",
                    5,
                ));
            }
            let value = packet.get("data").cloned().ok_or_else(AppError::protocol)?;
            if value.get("stream_id").and_then(Value::as_str)
                != payload.get("stream_id").and_then(Value::as_str)
            {
                return Err(AppError::protocol());
            }
            data = Some(value);
            Ok(())
        },
    )?;
    data.ok_or_else(AppError::protocol)
}
fn count(data: &Value, key: &str) -> Result<u64> {
    data.get(key)
        .and_then(Value::as_u64)
        .ok_or_else(AppError::protocol)
}
fn ack(data: &Value, status: &str, received: u64, total: u64) -> Result<()> {
    if data.get("type").and_then(Value::as_str)
        != Some(if status == "file_complete" {
            "response"
        } else {
            "stream"
        })
        || data.get("status").and_then(Value::as_str) != Some(status)
        || count(data, "received_chunks")? != received
        || count(data, "total_chunks")? != total
    {
        return Err(AppError::protocol());
    }
    Ok(())
}
pub(super) fn offline(command: &FileCommand) -> Result<Option<Envelope>> {
    let FileCommand::Stage { file, write } = command else {
        return Ok(None);
    };
    if !write.dry_run {
        return Ok(None);
    }
    attachments::validate_source(file)?;
    Ok(Some(Envelope::typed(StageDraft {
        operation: StageOperation::Stage,
        dry_run: Yes,
        identity_verified: No,
    })))
}
pub(super) fn execute<B: Backend>(
    command: &FileCommand,
    session: &Session<'_, B>,
    store: &Store,
    references: Option<&References>,
    deadline: Instant,
) -> Result<Option<Envelope>> {
    let FileCommand::Stage { file, write } = command else {
        return Ok(None);
    };
    session.authorize(None, true)?;
    let refs = references.ok_or_else(AppError::storage)?;
    let snapshot = attachments::prepare_for_run(
        file,
        attachments::Kind::File,
        store,
        session.instance,
        deadline,
    )?;
    let fingerprint = snapshot.fingerprint();
    let key = write.request_id.as_deref().ok_or_else(AppError::invalid)?;
    let scope = Scope {
        instance_id: session.instance.id.clone(),
        account: session
            .instance
            .account
            .clone()
            .ok_or_else(AppError::protocol)?,
    };
    let binding = refs
        .api_fingerprint(&serde_json::to_value(&fingerprint).map_err(|_| AppError::invalid())?)?;
    let plan = json!({"action":"upload_file_stream","stageBinding":binding,"retentionMs":300000});
    let intent = match ledger::begin(
        store,
        key,
        "api.upload_file_stream",
        &scope,
        &plan,
        deadline,
    )? {
        Begin::Existing(record) => {
            if record.receipt.is_some() {
                return Err(AppError::storage());
            }
            if record.outcome == Outcome::Rejected {
                let mut error = AppError::new(
                    "REQUEST_REJECTED",
                    "The original upload was rejected; this key was not sent again.",
                    4,
                );
                error.outcome = Some(Outcome::Rejected);
                error.request_id = Some(key.into());
                return Err(error.with_scope(record.scope));
            }
            if record.outcome != Outcome::Accepted {
                return Err(ledger::unknown(key).with_scope(record.scope));
            }
            return Ok(Some(
                Envelope::typed(StageReceipt {
                    operation: StageOperation::Stage,
                    request_id: LocalId::new(key)?,
                    outcome: record.outcome,
                    deduplicated: true,
                    native_path: None,
                    path_available: false,
                    size_bytes: fingerprint.size_bytes,
                    sha256: fingerprint.sha256,
                    requested_retention_seconds: 300,
                })
                .scoped(record.scope),
            ));
        }
        Begin::Fresh(intent) => intent,
    };
    if let Err(mut error) = session.verify(deadline) {
        intent.finish(Outcome::Rejected, None)?;
        error.outcome = Some(Outcome::Rejected);
        error.request_id = Some(key.into());
        return Err(error.with_scope(scope));
    }
    let retained = match snapshot.retain_for_intent(&intent, refs, deadline) {
        Ok(file) => file,
        Err(mut error) => {
            intent.finish(Outcome::Rejected, None)?;
            error.outcome = Some(Outcome::Rejected);
            error.request_id = Some(key.into());
            return Err(error.with_scope(scope));
        }
    };
    let result = (|| {
        let id=format!("qq-cli-{}",refs.api_fingerprint(&json!({"request":key,"run":session.instance.run_id,"binding":binding,"scope":scope}))?);
        let filename = format!("{id}.bin");
        let total = fingerprint.size_bytes.div_ceil(CHUNK as u64);
        let mut create = payload(&id);
        create.filename = Some(filename.clone());
        create.total_chunks = Some(total as f64);
        create.file_size = Some(fingerprint.size_bytes as f64);
        create.expected_sha256 = Some(fingerprint.sha256.clone());
        let created = call(session, &create, deadline)?;
        ack(&created, "file_created", 0, total)?;
        let mut input = std::fs::File::open(retained.path()).map_err(|_| AppError::storage())?;
        let mut remaining = fingerprint.size_bytes;
        let mut bytes = vec![0_u8; CHUNK];
        for index in 0..total {
            crate::core::deadline::check(deadline)?;
            let size = remaining.min(CHUNK as u64) as usize;
            input
                .read_exact(&mut bytes[..size])
                .map_err(|_| AppError::storage())?;
            let mut chunk = payload(&id);
            chunk.chunk_index = Some(index as f64);
            chunk.chunk_data = Some(STANDARD.encode(&bytes[..size]));
            let result = call(session, &chunk, deadline)?;
            ack(&result, "chunk_received", index + 1, total)?;
            remaining -= size as u64;
        }
        let mut complete = payload(&id);
        complete.is_complete = Some(true);
        let result = call(session, &complete, deadline)?;
        ack(&result, "file_complete", total, total)?;
        if result.get("type").and_then(Value::as_str) != Some("response")
            || count(&result, "file_size")? != fingerprint.size_bytes
            || result.get("sha256").and_then(Value::as_str) != Some(&fingerprint.sha256)
        {
            return Err(AppError::protocol());
        }
        let path = result
            .get("file_path")
            .and_then(Value::as_str)
            .filter(|path| path.rsplit(['/', '\\']).next() == Some(filename.as_str()))
            .ok_or_else(AppError::protocol)?
            .to_owned();
        session.verify(deadline)?;
        Ok(path)
    })();
    let native_path = match result {
        Ok(path) => {
            intent.finish(Outcome::Accepted, None)?;
            path
        }
        Err(_) => {
            intent.finish(Outcome::Unknown, None)?;
            return Err(ledger::unknown(key).with_scope(scope));
        }
    };
    Ok(Some(
        Envelope::typed(StageReceipt {
            operation: StageOperation::Stage,
            request_id: LocalId::new(key)?,
            outcome: Outcome::Accepted,
            deduplicated: false,
            native_path: Some(native_path),
            path_available: true,
            size_bytes: fingerprint.size_bytes,
            sha256: fingerprint.sha256,
            requested_retention_seconds: 300,
        })
        .scoped(scope),
    ))
}

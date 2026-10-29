//! Native chunks remain temporary until source, counts and final identity are confirmed.
use super::*;
use crate::core::contracts::{
    streaming::{DownloadPayload, RecordPayload},
    voice::Format,
    write::{Download, DownloadKind},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::io::{Seek, SeekFrom};
struct Chunks {
    file: File,
    info: bool,
    complete: bool,
    count: u64,
    size: u64,
    chunk_size: u64,
    declared_size: Option<u64>,
    converted: bool,
    format: &'static str,
}
fn number(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(AppError::protocol)
}
impl Chunks {
    fn accept(&mut self, raw: Value) -> Result<()> {
        if self.complete {
            return Err(AppError::protocol());
        }
        if raw.get("status").and_then(Value::as_str) != Some("ok")
            || raw.get("retcode").and_then(Value::as_i64) != Some(0)
        {
            return Err(AppError::new(
                "BACKEND_REJECTED",
                "Native file stream reported failure; no output file was published.",
                4,
            ));
        }
        let data = raw.get("data").ok_or_else(AppError::protocol)?;
        match (
            data.get("type").and_then(Value::as_str),
            data.get("data_type").and_then(Value::as_str),
        ) {
            (Some("stream"), Some("file_info")) if !self.info => {
                let size = number(data, "file_size")?;
                if size > MAX_DOWNLOAD_BYTES {
                    return Err(bytes_error());
                }
                self.declared_size = (!self.converted).then_some(size);
                self.chunk_size = number(data, "chunk_size")?;
                if self.chunk_size == 0 || self.chunk_size > 65536 {
                    return Err(AppError::protocol());
                }
                if self.converted
                    && data.get("out_format").and_then(Value::as_str) != Some(self.format)
                {
                    return Err(AppError::protocol());
                }
                self.info = true;
            }
            (Some("stream"), Some("file_chunk")) if self.info => {
                if number(data, "index")? != self.count {
                    return Err(AppError::protocol());
                }
                let text = data
                    .get("data")
                    .and_then(Value::as_str)
                    .ok_or_else(AppError::protocol)?;
                if text.len() > ((self.chunk_size as usize).div_ceil(3) * 4)
                    || number(data, "base64_size")? != text.len() as u64
                {
                    return Err(AppError::protocol());
                }
                let bytes = STANDARD.decode(text).map_err(|_| AppError::protocol())?;
                if bytes.is_empty()
                    || bytes.len() as u64 > self.chunk_size
                    || number(data, "size")? != bytes.len() as u64
                {
                    return Err(AppError::protocol());
                }
                self.size += bytes.len() as u64;
                if self.size > MAX_DOWNLOAD_BYTES {
                    return Err(bytes_error());
                }
                self.file
                    .write_all(&bytes)
                    .map_err(|_| AppError::storage())?;
                self.count += 1;
            }
            (Some("response"), Some("file_complete")) if self.info => {
                if number(data, "total_chunks")? != self.count
                    || number(data, "total_bytes")? != self.size
                    || self.declared_size.is_some_and(|size| size != self.size)
                {
                    return Err(changed());
                }
                self.complete = true;
            }
            _ => return Err(AppError::protocol()),
        }
        Ok(())
    }
}
pub fn download<B: Backend>(
    token: &str,
    output: &Path,
    format: Format,
    session: &Session<'_, B>,
    references: &References,
    deadline: Instant,
) -> Result<crate::core::output::Envelope> {
    output_path(output)?;
    let saved: MediaLocator = references.get("f1", token)?;
    if saved.kind == "image" {
        return super::download_as(token, output, format, session, references, deadline);
    }
    if format != Format::Original && saved.kind != "record" {
        return Err(AppError::invalid());
    }
    let locator = verify_reference(token, session, references, deadline)?;
    if !locator.opaque_id || !matches!(locator.kind.as_str(), "file" | "record") {
        return Err(AppError::new(
            "MEDIA_BINDING_UNVERIFIED",
            "Native file streaming requires an exact source attachment alias.",
            4,
        ));
    }
    if locator.size.is_some_and(|size| size > MAX_DOWNLOAD_BYTES) {
        return Err(bytes_error());
    }
    let converted = locator.kind == "record" && format != Format::Original;
    let payload = DownloadPayload {
        file: None,
        file_id: Some(locator.file_id.clone()),
        chunk_size: Some(65536.0),
    };
    let (action, payload) = if locator.kind == "record" {
        (
            "download_file_record_stream",
            serde_json::to_value(RecordPayload {
                download: payload,
                out_format: converted.then(|| format.wire().into()),
            }),
        )
    } else {
        ("download_file_stream", serde_json::to_value(payload))
    };
    let payload = payload.map_err(|_| AppError::protocol())?;
    let mut chunks = Chunks {
        file: tempfile::tempfile().map_err(|_| AppError::storage())?,
        info: false,
        complete: false,
        count: 0,
        size: 0,
        chunk_size: 0,
        declared_size: None,
        converted,
        format: format.wire(),
    };
    let (_, scope) = session.read_in_scope(
        Some(&locator.message.conversation),
        Some(&locator.message.scope),
        deadline,
        |backend| {
            backend.call_stream(
                &action.parse().map_err(|_| AppError::protocol())?,
                &payload,
                deadline,
                &mut |packet| chunks.accept(packet),
            )
        },
    )?;
    if !chunks.complete || (!converted && locator.size.is_some_and(|size| size != chunks.size)) {
        return Err(changed());
    }
    chunks
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|_| AppError::storage())?;
    let transfer = publish(chunks.file, output, Some(chunks.size), deadline, || {
        session.verify(deadline).map(|_| ())
    })?;
    Ok(crate::core::output::Envelope::typed(Download {
        media_ref: token.into(),
        kind: if locator.kind == "record" {
            DownloadKind::Record
        } else {
            DownloadKind::File
        },
        size_bytes: transfer.size_bytes,
        sha256: transfer.sha256,
    })
    .scoped(scope))
}

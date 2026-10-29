//! Incremental OneBot HTTP JSON records; packets are bounded before allocating JSON values.
use crate::core::error::{AppError, Result};
use serde_json::{Value, value::RawValue};
pub const MAX_WIRE_BYTES: usize = 192 * 1024 * 1024;
pub const MAX_RECORDS: usize = 4096;
pub(crate) struct Decoder {
    buffer: Vec<u8>,
    limit: usize,
    records: usize,
}
impl Decoder {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            buffer: Vec::new(),
            limit,
            records: 0,
        }
    }
    pub(crate) fn feed(
        &mut self,
        bytes: &[u8],
        receive: &mut dyn FnMut(Value) -> Result<()>,
    ) -> Result<()> {
        self.buffer.extend_from_slice(bytes);
        loop {
            let mut parsed =
                serde_json::Deserializer::from_slice(&self.buffer).into_iter::<&RawValue>();
            match parsed.next() {
                Some(Ok(raw)) => {
                    let consumed = parsed.byte_offset();
                    if raw.get().len() > self.limit || self.records >= MAX_RECORDS {
                        return Err(limit());
                    }
                    let value: Value =
                        serde_json::from_str(raw.get()).map_err(|_| AppError::protocol())?;
                    if !value.is_object()
                        || value.get("status").and_then(Value::as_str).is_none()
                        || value.get("retcode").and_then(Value::as_i64).is_none()
                    {
                        return Err(AppError::protocol());
                    }
                    self.records += 1;
                    receive(value)?;
                    self.buffer.drain(..consumed);
                }
                Some(Err(error)) if !error.is_eof() => return Err(AppError::protocol()),
                _ => {
                    if self.buffer.len() > self.limit {
                        return Err(limit());
                    }
                    return Ok(());
                }
            }
        }
    }
    pub(crate) fn finish(&self) -> Result<()> {
        if self.records == 0 || !self.buffer.iter().all(u8::is_ascii_whitespace) {
            return Err(AppError::protocol());
        }
        Ok(())
    }
}
fn limit() -> AppError {
    AppError::new(
        "STREAM_LIMIT",
        "Native stream exceeded its record budget.",
        3,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_accept_split_utf8_escaped_strings_and_blank_line_or_eof_framing() {
        // Build valid wire data with the serializer, including embedded newlines and UTF-8.
        let mut wire = serde_json::to_vec(
            &serde_json::json!({"status":"ok","retcode":0,"data":{"text":"中文\n\"nested"}}),
        )
        .unwrap();
        wire.extend_from_slice(b"\r\n\r\n");
        wire.extend_from_slice(br#"{"status":"failed","retcode":200,"data":null}"#);
        for size in 1..=wire.len() {
            let mut decoder = Decoder::new(4096);
            let mut records = Vec::new();
            for chunk in wire.chunks(size) {
                decoder
                    .feed(chunk, &mut |v| {
                        records.push(v);
                        Ok(())
                    })
                    .unwrap();
            }
            decoder.finish().unwrap();
            assert_eq!(records.len(), 2);
            assert_eq!(records[0]["data"]["text"], "中文\n\"nested");
        }
    }
    #[test]
    fn truncated_non_object_oversized_and_unbounded_records_are_rejected() {
        let mut d = Decoder::new(16);
        assert!(
            d.feed(br#"{"status":"ok","retcode":0}"#, &mut |_| Ok(()))
                .is_err()
        );
        for wire in [b"null".as_slice(), b"[]", b"{bad"] {
            assert!(Decoder::new(100).feed(wire, &mut |_| Ok(())).is_err());
        }
        let mut d = Decoder::new(100);
        d.feed(b"{", &mut |_| Ok(())).unwrap();
        assert!(d.finish().is_err());
        let mut d = Decoder::new(100);
        let wire = br#"{"status":"ok","retcode":0}"#;
        for _ in 0..MAX_RECORDS {
            d.feed(wire, &mut |_| Ok(())).unwrap();
        }
        assert!(d.feed(wire, &mut |_| Ok(())).is_err());
    }
}

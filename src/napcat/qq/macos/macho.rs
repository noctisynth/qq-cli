//! Bounded Mach-O header/command inspection, not code-signature validation.
use super::super::metadata_error;
use crate::core::error::Result;
use std::io::{Read, Seek, SeekFrom};

const ARM64: u32 = 0x0100000c;
fn read<const N: usize>(file: &mut (impl Read + Seek), at: u64) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    file.seek(SeekFrom::Start(at))
        .map_err(|_| metadata_error())?;
    file.read_exact(&mut bytes).map_err(|_| metadata_error())?;
    Ok(bytes)
}
fn be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes.try_into().unwrap())
}
fn le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}

pub(super) fn arm64(file: &mut (impl Read + Seek), size: u64, expected_type: u32) -> Result<()> {
    let header = read::<8>(file, 0)?;
    if header[..4] == [0xcf, 0xfa, 0xed, 0xfe] {
        return thin(file, 0, size, expected_type);
    }
    let width = match be(&header[..4]) {
        0xcafebabe => 20,
        0xcafebabf => 32,
        _ => return Err(metadata_error()),
    };
    let count = be(&header[4..]);
    if count == 0 || count > 16 {
        return Err(metadata_error());
    }
    let table_end = 8 + u64::from(count) * width;
    let mut ranges = Vec::new();
    let mut selected = None;
    for index in 0..count {
        let at = 8 + u64::from(index) * width;
        let mut record = [0_u8; 32];
        file.seek(SeekFrom::Start(at))
            .map_err(|_| metadata_error())?;
        file.read_exact(&mut record[..width as usize])
            .map_err(|_| metadata_error())?;
        let (offset, length, alignment) = if width == 20 {
            (
                u64::from(be(&record[8..12])),
                u64::from(be(&record[12..16])),
                be(&record[16..20]),
            )
        } else {
            if be(&record[28..32]) != 0 {
                return Err(metadata_error());
            }
            (
                u64::from_be_bytes(record[8..16].try_into().unwrap()),
                u64::from_be_bytes(record[16..24].try_into().unwrap()),
                be(&record[24..28]),
            )
        };
        let end = offset.checked_add(length).ok_or_else(metadata_error)?;
        if offset < table_end
            || length < 32
            || end > size
            || alignment > 31
            || offset % (1_u64 << alignment) != 0
            || ranges
                .iter()
                .any(|&(start, stop)| offset < stop && end > start)
        {
            return Err(metadata_error());
        }
        ranges.push((offset, end));
        if be(&record[..4]) == ARM64
            && selected
                .replace((offset, length, be(&record[4..8])))
                .is_some()
        {
            return Err(metadata_error());
        }
    }
    let (offset, length, subtype) = selected.ok_or_else(metadata_error)?;
    let header = read::<12>(file, offset)?;
    if le(&header[8..12]) != subtype {
        return Err(metadata_error());
    }
    thin(file, offset, length, expected_type)
}

fn thin(file: &mut (impl Read + Seek), offset: u64, size: u64, expected_type: u32) -> Result<()> {
    if size < 32 {
        return Err(metadata_error());
    }
    let header = read::<32>(file, offset)?;
    let commands = le(&header[16..20]);
    let bytes = u64::from(le(&header[20..24]));
    if header[..4] != [0xcf, 0xfa, 0xed, 0xfe]
        || le(&header[4..8]) != ARM64
        || le(&header[8..12]) & 0x00ffffff > 1
        || le(&header[12..16]) != expected_type
        || commands > 4096
        || bytes > 16 * 1024 * 1024
        || 32 + bytes > size
        || le(&header[28..32]) != 0
    {
        return Err(metadata_error());
    }
    let mut position = 32_u64;
    for _ in 0..commands {
        if position + 8 > 32 + bytes {
            return Err(metadata_error());
        }
        let command = read::<8>(file, offset + position)?;
        let length = u64::from(le(&command[4..]));
        if length < 8 || length % 8 != 0 || position + length > 32 + bytes {
            return Err(metadata_error());
        }
        position += length;
    }
    if position != 32 + bytes {
        return Err(metadata_error());
    }
    Ok(())
}

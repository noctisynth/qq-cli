//! Bounded Mach-O header/command inspection, not code-signature validation.
use super::super::metadata_error;
use crate::core::error::Result;
use std::io::{Read, Seek, SeekFrom};

mod signature;
pub(crate) use signature::ResignPayload;

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
    if arm64_kind(file, size)? != expected_type {
        return Err(metadata_error());
    }
    Ok(())
}

pub(crate) fn arm64_kind(file: &mut (impl Read + Seek), size: u64) -> Result<u32> {
    let layout = layout(file, size)?;
    let mut native = layout.slices.iter().filter(|slice| slice.cpu == ARM64);
    let slice = native.next().ok_or_else(metadata_error)?;
    if native.next().is_some() {
        return Err(metadata_error());
    }
    let header = read::<12>(file, slice.offset)?;
    if le(&header[8..12]) != slice.subtype {
        return Err(metadata_error());
    }
    thin(file, slice.offset, slice.size)
}

struct Slice {
    cpu: u32,
    subtype: u32,
    offset: u64,
    size: u64,
    alignment: u32,
}
struct Layout {
    format: u32,
    table_end: u64,
    slices: Vec<Slice>,
}
fn layout(file: &mut (impl Read + Seek), size: u64) -> Result<Layout> {
    if size < 32 {
        return Err(metadata_error());
    }
    let header = read::<12>(file, 0)?;
    if header[..4] == [0xcf, 0xfa, 0xed, 0xfe] {
        return Ok(Layout {
            format: 0,
            table_end: 0,
            slices: vec![Slice {
                cpu: le(&header[4..8]),
                subtype: le(&header[8..12]),
                offset: 0,
                size,
                alignment: 0,
            }],
        });
    }
    let format = be(&header[..4]);
    let width = match format {
        0xcafebabe => 20,
        0xcafebabf => 32,
        _ => return Err(metadata_error()),
    };
    let count = be(&header[4..8]);
    if count == 0 || count > 16 {
        return Err(metadata_error());
    }
    let table_end = 8 + u64::from(count) * width;
    let mut slices: Vec<Slice> = Vec::new();
    for index in 0..count {
        let mut record = [0_u8; 32];
        file.seek(SeekFrom::Start(8 + u64::from(index) * width))
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
        let cpu = be(&record[..4]);
        let subtype = be(&record[4..8]);
        if offset < table_end
            || length < 32
            || end > size
            || alignment > 31
            || offset % (1_u64 << alignment) != 0
            || slices.iter().any(|s| {
                (offset < s.offset + s.size && end > s.offset)
                    || (cpu == s.cpu && subtype == s.subtype)
            })
        {
            return Err(metadata_error());
        }
        slices.push(Slice {
            cpu,
            subtype,
            offset,
            size: length,
            alignment,
        });
    }
    Ok(Layout {
        format,
        table_end,
        slices,
    })
}

fn thin(file: &mut (impl Read + Seek), offset: u64, size: u64) -> Result<u32> {
    if size < 32 {
        return Err(metadata_error());
    }
    let header = read::<32>(file, offset)?;
    let commands = le(&header[16..20]);
    let bytes = u64::from(le(&header[20..24]));
    if header[..4] != [0xcf, 0xfa, 0xed, 0xfe]
        || le(&header[4..8]) != ARM64
        || le(&header[8..12]) & 0x00ffffff > 1
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
    Ok(le(&header[12..16]))
}

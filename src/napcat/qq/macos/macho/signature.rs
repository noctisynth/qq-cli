//! Fingerprint code/data while allowing a bounded existing signature replacement.
//! Signature validity, publisher trust and entitlements remain independent checks.
use super::*;
use sha2::{Digest, Sha256};
use std::time::Instant;

#[cfg(test)]
pub(super) mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResignPayload([u8; 32]);

impl ResignPayload {
    pub(crate) fn inspect(
        file: &mut (impl Read + Seek),
        size: u64,
        deadline: Instant,
    ) -> Result<Self> {
        crate::core::deadline::check(deadline)?;
        if size > 2 * 1024 * 1024 * 1024 {
            return Err(metadata_error());
        }
        let layout = layout(file, size)?;
        let mut digest = Sha256::new();
        digest.update(b"qq-cli-macos-resign-payload-v1\0");
        digest.update(layout.format.to_le_bytes());
        digest.update((layout.slices.len() as u32).to_le_bytes());
        for slice in &layout.slices {
            for value in [slice.cpu, slice.subtype, slice.alignment] {
                digest.update(value.to_le_bytes());
            }
            digest.update(payload(file, slice, deadline)?);
        }
        if layout.format != 0 {
            let mut ordered: Vec<_> = layout.slices.iter().collect();
            ordered.sort_by_key(|slice| slice.offset);
            let mut position = layout.table_end;
            for slice in ordered {
                zeros(file, position, slice.offset - position, deadline)?;
                position = slice.offset + slice.size;
            }
            zeros(file, position, size - position, deadline)?;
        }
        Ok(Self(digest.finalize().into()))
    }
}

fn word64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap())
}
fn payload(file: &mut (impl Read + Seek), slice: &Slice, deadline: Instant) -> Result<[u8; 32]> {
    crate::core::deadline::check(deadline)?;
    let header = read::<32>(file, slice.offset)?;
    let count = le(&header[16..20]);
    let command_bytes = le(&header[20..24]) as usize;
    if header[..4] != [0xcf, 0xfa, 0xed, 0xfe]
        || ![ARM64, 0x01000007].contains(&slice.cpu)
        || le(&header[4..8]) != slice.cpu
        || le(&header[8..12]) != slice.subtype
        || ![2, 6, 8].contains(&le(&header[12..16]))
        || le(&header[28..32]) != 0
        || count > 4096
        || command_bytes > 16 * 1024 * 1024
        || 32 + command_bytes as u64 > slice.size
    {
        return Err(metadata_error());
    }
    let mut commands = vec![0; command_bytes];
    file.read_exact(&mut commands)
        .map_err(|_| metadata_error())?;
    let mut at = 0;
    let mut signature = None;
    let mut link = None;
    let mut file_end = 0;
    let mut vm_end = 0;
    for _ in 0..count {
        if at + 8 > commands.len() {
            return Err(metadata_error());
        }
        let cmd = le(&commands[at..at + 4]);
        let length = le(&commands[at + 4..at + 8]) as usize;
        if length < 8 || !length.is_multiple_of(8) || length > commands.len() - at {
            return Err(metadata_error());
        }
        if cmd == 0x1d {
            if length != 16 || signature.is_some() {
                return Err(metadata_error());
            }
            signature = Some((
                u64::from(le(&commands[at + 8..at + 12])),
                u64::from(le(&commands[at + 12..at + 16])),
            ));
            commands[at + 12..at + 16].fill(0);
        }
        if cmd == 0x19 {
            if length < 72 {
                return Err(metadata_error());
            }
            let vmaddr = word64(&commands[at + 24..at + 32]);
            let vmsize = word64(&commands[at + 32..at + 40]);
            let fileoff = word64(&commands[at + 40..at + 48]);
            let filesize = word64(&commands[at + 48..at + 56]);
            let end = fileoff.checked_add(filesize).ok_or_else(metadata_error)?;
            let vmstop = vmaddr.checked_add(vmsize).ok_or_else(metadata_error)?;
            let sections = le(&commands[at + 64..at + 68]) as usize;
            if sections > 4096 || length != 72 + sections * 80 || end > slice.size {
                return Err(metadata_error());
            }
            if &commands[at + 8..at + 24] == b"__LINKEDIT\0\0\0\0\0\0" {
                if link.is_some()
                    || sections != 0
                    || le(&commands[at + 56..at + 60]) != 1
                    || le(&commands[at + 60..at + 64]) != 1
                    || le(&commands[at + 68..at + 72]) != 0
                    || vmsize < filesize
                    || !vmsize.is_multiple_of(if slice.cpu == ARM64 { 16384 } else { 4096 })
                    || vmsize
                        > filesize.checked_add(16383).ok_or_else(metadata_error)? / 16384 * 16384
                {
                    return Err(metadata_error());
                }
                link = Some((vmaddr, fileoff, end));
                commands[at + 32..at + 40].fill(0);
                commands[at + 48..at + 56].fill(0);
            } else {
                if filesize != 0 {
                    file_end = file_end.max(end);
                }
                vm_end = vm_end.max(vmstop);
            }
        }
        at += length;
    }
    if at != commands.len() {
        return Err(metadata_error());
    }
    let (offset, length) = signature.ok_or_else(metadata_error)?;
    let (vmaddr, fileoff, end) = link.ok_or_else(metadata_error)?;
    if offset < 32 + command_bytes as u64
        || offset < fileoff
        || file_end > fileoff
        || vm_end > vmaddr
        || end != slice.size
        || offset.checked_add(length) != Some(slice.size)
        || !(12..=64 * 1024 * 1024).contains(&length)
    {
        return Err(metadata_error());
    }
    let sig = read::<12>(file, slice.offset + offset)?;
    let declared = u64::from(be(&sig[4..8]));
    if be(&sig[..4]) != 0xfade0cc0 || declared < 12 || declared > length {
        return Err(metadata_error());
    }
    // codesign can retain nonzero allocation padding after the declared
    // SuperBlob length. The entire bounded LC_CODE_SIGNATURE allocation is
    // mutable; signature validity must be checked separately by the OS.
    let mut hash = Sha256::new();
    hash.update(header);
    hash.update(&commands);
    stream(
        file,
        slice.offset + 32 + command_bytes as u64,
        offset - 32 - command_bytes as u64,
        deadline,
        |bytes| {
            hash.update(bytes);
            Ok(())
        },
    )?;
    Ok(hash.finalize().into())
}

fn zeros(file: &mut (impl Read + Seek), offset: u64, length: u64, deadline: Instant) -> Result<()> {
    stream(file, offset, length, deadline, |bytes| {
        if bytes.iter().any(|b| *b != 0) {
            Err(metadata_error())
        } else {
            Ok(())
        }
    })
}
fn stream(
    file: &mut (impl Read + Seek),
    offset: u64,
    mut length: u64,
    deadline: Instant,
    mut accept: impl FnMut(&[u8]) -> Result<()>,
) -> Result<()> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| metadata_error())?;
    let mut bytes = vec![0; 65536];
    while length != 0 {
        crate::core::deadline::check(deadline)?;
        let n = length.min(bytes.len() as u64) as usize;
        file.read_exact(&mut bytes[..n])
            .map_err(|_| metadata_error())?;
        accept(&bytes[..n])?;
        length -= n as u64;
    }
    crate::core::deadline::check(deadline)
}

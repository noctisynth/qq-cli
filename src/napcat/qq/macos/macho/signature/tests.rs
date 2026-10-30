use super::*;
use std::{io::Cursor, time::Duration};

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put64(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
// Structural fixture only: its SuperBlob is deliberately not a valid signature.
pub(crate) fn thin_fixture(signature_size: usize) -> Vec<u8> {
    let mut bytes = vec![0; 256 + signature_size];
    for (at, value) in [
        (0, 0xfeedfacf),
        (4, ARM64),
        (12, 2),
        (16, 2),
        (20, 88),
        (32, 0x19),
        (36, 72),
        (88, 1),
        (92, 1),
        (104, 0x1d),
        (108, 16),
        (112, 256),
        (116, signature_size as u32),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[40..56].copy_from_slice(b"__LINKEDIT\0\0\0\0\0\0");
    put64(&mut bytes, 56, 16384);
    put64(&mut bytes, 64, 16384);
    put64(&mut bytes, 72, 128);
    put64(&mut bytes, 80, (128 + signature_size) as u64);
    bytes[256..260].copy_from_slice(&0xfade0cc0_u32.to_be_bytes());
    bytes[260..264].copy_from_slice(&12_u32.to_be_bytes());
    bytes
}
fn inspect(bytes: &[u8]) -> Result<ResignPayload> {
    ResignPayload::inspect(
        &mut Cursor::new(bytes),
        bytes.len() as u64,
        Instant::now() + Duration::from_secs(2),
    )
}

#[test]
fn only_signature_allocation_and_bounded_linkedit_sizes_are_mutable() {
    let original = thin_fixture(16);
    let baseline = inspect(&original).unwrap();
    let mut resized = thin_fixture(64);
    resized[268..].fill(0xa5); // Includes nonzero signature allocation padding.
    assert!(baseline == inspect(&resized).unwrap());
    for at in [24, 56, 128, 255] {
        let mut changed = original.clone();
        changed[at] ^= 1;
        assert!(inspect(&changed).is_ok_and(|value| value != baseline));
    }
}

#[test]
fn malformed_signature_and_linkedit_layouts_are_rejected() {
    let original = thin_fixture(16);
    for (at, value) in [
        (16, 3),
        (20, 96),
        (28, 1),
        (36, 64),
        (88, 7),
        (92, 3),
        (96, 1),
        (100, 1),
        (104, 0),
        (108, 8),
        (112, 119),
        (112, 257),
        (116, 11),
        (116, 17),
    ] {
        let mut changed = original.clone();
        put32(&mut changed, at, value);
        assert!(inspect(&changed).is_err(), "accepted field {at}={value}");
    }
    for (at, value) in [(64, 32768), (64, 128), (72, 257), (80, 143)] {
        let mut changed = original.clone();
        put64(&mut changed, at, value);
        assert!(inspect(&changed).is_err());
    }
    for (at, value) in [(256, 0_u32), (260, 11), (260, 17)] {
        let mut changed = original.clone();
        changed[at..at + 4].copy_from_slice(&value.to_be_bytes());
        assert!(inspect(&changed).is_err());
    }
    assert!(inspect(&original[..original.len() - 1]).is_err());
}

fn fat64(offset: usize, signature_size: usize) -> Vec<u8> {
    let thin = thin_fixture(signature_size);
    let mut bytes = vec![0; offset + thin.len()];
    bytes[..4].copy_from_slice(&0xcafebabf_u32.to_be_bytes());
    bytes[4..8].copy_from_slice(&1_u32.to_be_bytes());
    bytes[8..12].copy_from_slice(&ARM64.to_be_bytes());
    bytes[16..24].copy_from_slice(&(offset as u64).to_be_bytes());
    bytes[24..32].copy_from_slice(&(thin.len() as u64).to_be_bytes());
    bytes[32..36].copy_from_slice(&8_u32.to_be_bytes());
    bytes[offset..].copy_from_slice(&thin);
    bytes
}

#[test]
fn fat64_relayout_preserves_payload_and_rejects_hidden_padding_or_reserved_bits() {
    let original = fat64(256, 16);
    let baseline = inspect(&original).unwrap();
    assert!(baseline == inspect(&fat64(512, 64)).unwrap());
    for at in [36, 40, 255] {
        let mut changed = original.clone();
        changed[at] = 1;
        assert!(inspect(&changed).is_err());
    }
    let mut changed = original.clone();
    changed[256 + 128] = 1;
    assert!(baseline != inspect(&changed).unwrap());
    assert!(
        ResignPayload::inspect(
            &mut Cursor::new(&original),
            original.len() as u64,
            Instant::now() - Duration::from_secs(1)
        )
        .is_err()
    );
}

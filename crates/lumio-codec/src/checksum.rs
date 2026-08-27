//! Private 8-byte checksum. Not a public architecture algorithm ID.
//!
//! Mix is FNV-1a 64 (public domain): xor each octet, then wrapping multiply.
//! Digest bytes are little-endian so the same input hashes identically on every host.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn checksum_bytes(input: &[u8]) -> [u8; 8] {
    let mut hash = OFFSET;
    for &byte in input {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash.to_le_bytes()
}

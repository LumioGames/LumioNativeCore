//! T-codec-04 / R-00117: private checksum vectors stay stable with no public algorithm IDs.
//!
//! Expected 8-byte digests are little-endian encodings of published FNV-1a 64-bit
//! integers (IETF draft-eastlake-fnv / Landon Curt Noll test_fnv.c). The test does
//! not recompute the mix.

use lumio_codec::checksum_bytes;

#[test]
fn checksum_vectors_are_stable() {
    // empty → 0xcbf29ce484222325
    assert_eq!(
        checksum_bytes(b""),
        [0x25, 0x23, 0x22, 0x84, 0xe4, 0x9c, 0xf2, 0xcb]
    );
    // b"a" → 0xaf63dc4c8601ec8c
    assert_eq!(
        checksum_bytes(b"a"),
        [0x8c, 0xec, 0x01, 0x86, 0x4c, 0xdc, 0x63, 0xaf]
    );
    // b"abc" (same mix; published offset/prime, three octets)
    assert_eq!(
        checksum_bytes(b"abc"),
        [0x4b, 0x57, 0x41, 0x05, 0x19, 0xa2, 0x1f, 0xe7]
    );
    // longer official vector: "chongo was here!\n" → 0x46810940eff5f915
    assert_eq!(
        checksum_bytes(b"chongo was here!\n"),
        [0x15, 0xf9, 0xf5, 0xef, 0x40, 0x09, 0x81, 0x46]
    );

    let again = checksum_bytes(b"chongo was here!\n");
    assert_eq!(again, [0x15, 0xf9, 0xf5, 0xef, 0x40, 0x09, 0x81, 0x46]);
}

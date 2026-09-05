//! T-capability-01: StaticCapabilities stores a sorted unique key set.
//!
//! Keys are opaque embedder-owned `u32` values (ADR 0009); this test fixes
//! the set behaviour — ordering by raw value, duplicate rejection, membership
//! — and deliberately asserts nothing about what any particular key means.

use lumio_kernel::capability::{CapabilityKey, StaticCapabilities};
use lumio_kernel::error::ErrorCategory;

const A: CapabilityKey = CapabilityKey::from_raw(1);
const B: CapabilityKey = CapabilityKey::from_raw(2);
const C: CapabilityKey = CapabilityKey::from_raw(3);
const ABSENT: CapabilityKey = CapabilityKey::from_raw(4);

#[test]
fn static_capabilities_are_sorted_unique() {
    let caps =
        StaticCapabilities::from_keys([C, A, B]).expect("unsorted unique keys should be accepted");

    let collected: Vec<_> = caps.iter().collect();
    assert_eq!(
        collected,
        vec![A, B, C],
        "keys must come back in ascending raw-value order"
    );

    assert!(caps.contains(A));
    assert!(caps.contains(B));
    assert!(caps.contains(C));
    assert!(!caps.contains(ABSENT));

    let err = StaticCapabilities::from_keys([A, A]).expect_err("duplicate keys should be rejected");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

#[test]
fn key_round_trips_its_raw_value() {
    assert_eq!(CapabilityKey::from_raw(u32::MAX).raw(), u32::MAX);
    assert_eq!(A.raw(), 1);
    assert_ne!(A, B);
}

#[test]
fn empty_set_contains_nothing() {
    let caps = StaticCapabilities::from_keys([]).expect("empty set is valid");
    assert_eq!(caps.iter().count(), 0);
    assert!(!caps.contains(A));
}

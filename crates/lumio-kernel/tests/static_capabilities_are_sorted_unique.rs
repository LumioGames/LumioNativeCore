//! T-capability-01: StaticCapabilities stores a sorted unique key set.

use lumio_kernel::capability::{CapabilityKey, StaticCapabilities};
use lumio_kernel::error::ErrorCategory;

#[test]
fn static_capabilities_are_sorted_unique() {
    let caps = StaticCapabilities::from_keys([
        CapabilityKey::from_local_index(3),
        CapabilityKey::from_local_index(1),
        CapabilityKey::from_local_index(2),
    ])
    .expect("unsorted unique keys should be accepted");
    let collected: Vec<_> = caps.iter().collect();
    assert_eq!(
        collected,
        vec![
            CapabilityKey::from_local_index(1),
            CapabilityKey::from_local_index(2),
            CapabilityKey::from_local_index(3),
        ]
    );
    assert!(caps.contains(CapabilityKey::from_local_index(1)));
    assert!(caps.contains(CapabilityKey::from_local_index(2)));
    assert!(caps.contains(CapabilityKey::from_local_index(3)));
    assert!(!caps.contains(CapabilityKey::from_local_index(4)));

    let err = StaticCapabilities::from_keys([
        CapabilityKey::from_local_index(1),
        CapabilityKey::from_local_index(1),
    ])
    .expect_err("duplicate keys should be rejected");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

//! T-capability-01: StaticCapabilities stores a sorted unique key set, built
//! from keys projected out of the generated architecture registry.

use lumio_contract_types::registry;
use lumio_kernel::capability::{CapabilityKey, StaticCapabilities};
use lumio_kernel::error::ErrorCategory;

fn key(id: &str) -> CapabilityKey {
    CapabilityKey::from_registry_id(id).unwrap_or_else(|| panic!("registered capability {id}"))
}

#[test]
fn static_capabilities_are_sorted_unique() {
    let caps =
        StaticCapabilities::from_keys([key("ReferenceVoxel"), key("Native"), key("HybridCLR")])
            .expect("unsorted unique keys should be accepted");
    let collected: Vec<_> = caps.iter().collect();
    assert_eq!(
        collected,
        vec![key("Native"), key("HybridCLR"), key("ReferenceVoxel")],
        "keys must come back in registered-ordinal order"
    );
    assert!(caps.contains(key("Native")));
    assert!(caps.contains(key("HybridCLR")));
    assert!(caps.contains(key("ReferenceVoxel")));
    assert!(!caps.contains(key("VoxelSnapshot")));

    let err = StaticCapabilities::from_keys([key("Native"), key("Native")])
        .expect_err("duplicate keys should be rejected");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

/// The generated conversion half of T-capability-01: every published key maps
/// one-to-one onto a `CapabilityKey`, with no repository-private key table in
/// between (D-015 / ADR-040 §7.1). Same cross-check the architecture gate
/// runs, expressed against this repository's mapping.
#[test]
fn capability_keys_project_the_generated_registry() {
    let published: Vec<_> = registry::capability_keys().collect();
    assert!(
        !published.is_empty(),
        "the generated registry must publish capability keys"
    );

    let mapped: Vec<CapabilityKey> = published
        .iter()
        .copied()
        .map(CapabilityKey::from_registered)
        .collect();
    for (key, publication) in mapped.iter().zip(&published) {
        assert_eq!(key.as_registry_numeric(), publication.numeric());
        assert_eq!(
            CapabilityKey::from_registry_id(publication.id()),
            Some(*key),
            "id lookup must agree with the registered projection"
        );
    }

    // The whole published set is a valid static set: distinct keys, and every
    // one of them present after sorting.
    let caps = StaticCapabilities::from_keys(mapped.iter().copied())
        .expect("published keys are unique, so the full set must be accepted");
    assert_eq!(caps.iter().count(), published.len());
    for key in &mapped {
        assert!(caps.contains(*key));
    }

    assert_eq!(
        CapabilityKey::from_registry_id("NotARegisteredCapability"),
        None
    );
}

//! T-capability-04 / R-00086: CapabilitySource::require fails when the key is missing.

use lumio_kernel::capability::{
    CapabilityKey, CapabilitySource, ConfiguredLimits, StaticCapabilities,
};
use lumio_kernel::error::ErrorCategory;

const GRANTED: CapabilityKey = CapabilityKey::from_raw(1);
const WITHHELD: CapabilityKey = CapabilityKey::from_raw(2);

#[test]
fn require_missing_capability_fails() {
    let static_caps = StaticCapabilities::from_keys([GRANTED]).expect("unique keys");
    let limits = ConfiguredLimits {
        max_handles: 1,
        max_native_bytes: 1,
        max_jobs_queued: 1,
        max_jobs_running: 1,
        max_completion_items: 1,
    };
    let source = CapabilitySource::new(static_caps, limits).expect("valid limits");

    assert!(source.require(GRANTED).is_ok());

    let err = source.require(WITHHELD).expect_err("missing key must fail");
    assert_eq!(err.category(), ErrorCategory::CapabilityUnavailable);
}

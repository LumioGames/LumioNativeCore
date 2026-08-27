//! T-capability-02 / R-00084: ConfiguredLimits::validate rejects any zero field.

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::error::ErrorCategory;

#[test]
fn invalid_limits_are_rejected() {
    let ok = ConfiguredLimits {
        max_handles: 1,
        max_native_bytes: 1,
        max_jobs_queued: 1,
        max_jobs_running: 1,
        max_completion_items: 1,
    };
    assert!(ok.validate().is_ok());

    let mut zero_handles = ok;
    zero_handles.max_handles = 0;
    let err = zero_handles
        .validate()
        .expect_err("zero max_handles must fail");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);

    let mut zero_bytes = ok;
    zero_bytes.max_native_bytes = 0;
    zero_bytes
        .validate()
        .expect_err("zero max_native_bytes must fail");
}

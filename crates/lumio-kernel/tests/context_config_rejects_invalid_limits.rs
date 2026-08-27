//! T-context-01 / R-00099: ContextConfig::validate rejects invalid limits.

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::ContextConfig;
use lumio_kernel::error::ErrorCategory;
use lumio_platform::Deadline;

#[test]
fn context_config_rejects_invalid_limits() {
    let limits = ConfiguredLimits {
        max_handles: 1,
        max_native_bytes: 1,
        max_jobs_queued: 1,
        max_jobs_running: 1,
        max_completion_items: 1,
    };
    let ok = ContextConfig {
        limits,
        quiesce_deadline: Deadline::NONE,
    };
    assert!(ok.validate().is_ok());

    let mut zero_handles = ok;
    zero_handles.limits.max_handles = 0;
    let err = zero_handles
        .validate()
        .expect_err("zero max_handles must fail");
    assert_eq!(err.category(), ErrorCategory::InvalidArgument);
}

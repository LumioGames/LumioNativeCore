//! T-error-03 / R-00079: mapping covers every ErrorCategory and stays blocked.

use lumio_kernel::error::{
    ErrorCategory, ErrorDetail, KernelError, MappingBlocked, to_architecture_error_code,
};

#[test]
fn mapping_is_total_for_all_categories() {
    assert!(
        lumio_contract_types::registry::error_codes()
            .next()
            .is_none()
    );

    let categories = [
        ErrorCategory::InvalidArgument,
        ErrorCategory::InvalidHandle,
        ErrorCategory::WrongContext,
        ErrorCategory::AlreadyReleased,
        ErrorCategory::BufferTooSmall,
        ErrorCategory::CapacityExceeded,
        ErrorCategory::CapabilityUnavailable,
        ErrorCategory::Cancelled,
        ErrorCategory::TimedOut,
        ErrorCategory::ContextClosing,
        ErrorCategory::ContextDestroyed,
        ErrorCategory::PanicBoundary,
        ErrorCategory::InternalInvariant,
    ];

    for category in categories {
        let err = KernelError::new(category, ErrorDetail::None);
        assert_eq!(err.category(), category);
        assert_eq!(to_architecture_error_code(&err), Err(MappingBlocked));
    }
}

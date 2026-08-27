//! T-error-02: `KernelError::buffer_too_small` preserves required/provided.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

#[test]
fn buffer_too_small_reports_required() {
    let err = KernelError::buffer_too_small(64, 8);
    assert_eq!(err.category(), ErrorCategory::BufferTooSmall);
    match err.detail() {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 64);
            assert_eq!(*provided, 8);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
}

//! T-error-01: KernelError detail is a closed, non-owning payload.
//!
//! Exhaustive match is the bound: a `String` (or other owned) variant will not
//! compile until this test grows a new arm.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

fn assert_detail_is_bounded(detail: &ErrorDetail) {
    match detail {
        ErrorDetail::None => {}
        ErrorDetail::RequiredCapacity {
            required: _,
            provided: _,
        } => {}
        ErrorDetail::LimitExceeded {
            limit: _,
            requested: _,
        } => {}
        ErrorDetail::StaticMessage(_) => {}
    }
}

#[test]
fn kernel_error_detail_is_bounded() {
    let static_err = KernelError::new(
        ErrorCategory::InternalInvariant,
        ErrorDetail::StaticMessage("bounded-static"),
    );
    assert_eq!(static_err.category(), ErrorCategory::InternalInvariant);
    match static_err.detail() {
        ErrorDetail::StaticMessage(msg) => assert_eq!(*msg, "bounded-static"),
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_detail_is_bounded(static_err.detail());

    let capacity_err = KernelError::new(
        ErrorCategory::BufferTooSmall,
        ErrorDetail::RequiredCapacity {
            required: 32,
            provided: 8,
        },
    );
    assert_eq!(capacity_err.category(), ErrorCategory::BufferTooSmall);
    match capacity_err.detail() {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 32);
            assert_eq!(*provided, 8);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_detail_is_bounded(capacity_err.detail());
}

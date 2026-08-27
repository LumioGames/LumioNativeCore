//! Unified FFI panic/error boundary.
//!
//! `to_architecture_error_code` currently returns `Err(MappingBlocked)` for
//! every category, including `PanicBoundary` (T-error-03). Public numeric
//! Panic ErrorCode is blocked, so this seam returns `KernelError` rather than
//! `ArchitectureErrorCode`. After mapping is unblocked, FFI exports can call
//! `to_architecture_error_code` at the C ABI.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

/// Run `body` inside `catch_unwind`.
///
/// - `Ok(())` and `Err(e)` pass through unchanged.
/// - A panic becomes `ErrorCategory::PanicBoundary` and does not unwind.
pub fn ffi_boundary<F>(body: F) -> Result<(), KernelError>
where
    F: FnOnce() -> Result<(), KernelError> + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(body) {
        Ok(result) => result,
        Err(_) => Err(KernelError::new(
            ErrorCategory::PanicBoundary,
            ErrorDetail::None,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::ffi_boundary;
    use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError};

    #[test]
    fn panic_is_converted_and_does_not_unwind() {
        assert!(ffi_boundary(|| Ok(())).is_ok());

        let preserved = ffi_boundary(|| {
            Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ))
        });
        match preserved {
            Err(error) => assert_eq!(error.category(), ErrorCategory::InvalidArgument),
            Ok(()) => panic!("expected InvalidArgument to be preserved"),
        }

        let caught = std::panic::catch_unwind(|| ffi_boundary(|| panic!("ffi-test-panic")));
        match caught {
            Ok(Err(error)) => {
                assert_eq!(error.category(), ErrorCategory::PanicBoundary);
                assert_eq!(*error.detail(), ErrorDetail::None);
            }
            Ok(Ok(())) => panic!("panic body must become PanicBoundary"),
            Err(_) => panic!("ffi_boundary must catch panics and not unwind"),
        }
    }
}

//! Raw pointer/length checks for FFI inputs.
//!
//! Header layout, alignment, size, and version are blocked (T-ffi-02).
//! This helper only inspects null vs length and never dereferences.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

/// Accept or reject a raw buffer pointer and length without dereferencing.
///
/// - `len == 0`: `Ok`, including a null pointer.
/// - `len > 0` and `ptr` is null: `Err(InvalidArgument)`.
/// - `len > 0` and `ptr` is non-null: `Ok` (no layout/align/Header checks).
pub fn check_buffer_ptr_len(ptr: *const u8, len: usize) -> KernelResult<()> {
    if len > 0 && ptr.is_null() {
        return Err(KernelError::new(
            ErrorCategory::InvalidArgument,
            ErrorDetail::None,
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_buffer_ptr_len;
    use lumio_kernel::error::ErrorCategory;

    #[test]
    fn null_nonzero_length_is_rejected() {
        assert!(check_buffer_ptr_len(std::ptr::null(), 0).is_ok());

        match check_buffer_ptr_len(std::ptr::null(), 1) {
            Err(error) => assert_eq!(error.category(), ErrorCategory::InvalidArgument),
            Ok(()) => panic!("expected InvalidArgument for null pointer with nonzero length"),
        }

        let byte = 0u8;
        assert!(check_buffer_ptr_len(&byte as *const u8, 1).is_ok());
    }
}

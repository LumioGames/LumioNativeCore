//! T-ffi-02 / R-00124: null pointer with nonzero length is InvalidArgument.
//!
//! `lumio-native-ffi` is `cdylib`+`staticlib` only, so Cargo does not pass
//! `--extern lumio_native_ffi` (no rlib) to integration tests on this host.
//! The same `check_buffer_ptr_len` source is compiled via `#[path]`; the crate-root
//! `pub use` is covered by the `#[cfg(test)]` module in `validation.rs`.

#[path = "../src/validation.rs"]
mod validation;

use lumio_kernel::error::ErrorCategory;
use validation::check_buffer_ptr_len;

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

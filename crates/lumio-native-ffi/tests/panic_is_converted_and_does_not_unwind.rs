//! T-ffi-01 / R-00123: panics at the FFI boundary become PanicBoundary and do not unwind.
//!
//! `lumio-native-ffi` is `cdylib`+`staticlib` only, so Cargo does not pass
//! `--extern lumio_native_ffi` (no rlib) to integration tests on this host.
//! The same `ffi_boundary` source is compiled via `#[path]`; the crate-root
//! `pub use` is covered by the `#[cfg(test)]` module in `boundary.rs`.

#[path = "../src/boundary.rs"]
mod boundary;

use boundary::ffi_boundary;
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

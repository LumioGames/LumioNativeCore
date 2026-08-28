//! Blocked-header FFI smoke helper.
//!
//! The architecture source publishes the Root ABI C Header (ADR-040), but the
//! entry symbol it declares belongs to CoreEngine root-abi, not to this
//! repository (ADR 0001; enforced by `cargo xtask dump-symbols` and by the
//! source-text guard in this module's tests — do not spell that symbol here).
//! The provider symbol list this crate would export is still unpublished
//! (T-ffi-04).
//! This module composes existing Rust seams; it is not a C ABI surface.
//! Do not add `#[no_mangle]` or `extern "C"` names here.

use lumio_kernel::error::KernelError;
use lumio_kernel::handle::{ContextKey, HandleKey};

use crate::boundary::ffi_boundary;
use crate::handles::decode_handle_for_context;

/// Decode `key` for `expected` inside the FFI panic/error boundary.
///
/// Wrong-context handles return `ErrorCategory::WrongContext`. Mapping that
/// category to a public architecture ErrorCode remains `MappingBlocked`.
pub fn smoke_decode_handle(key: HandleKey, expected: ContextKey) -> Result<(), KernelError> {
    ffi_boundary(move || decode_handle_for_context(key, expected).map(|_| ()))
}

#[cfg(test)]
mod tests {
    use super::smoke_decode_handle;
    use crate::boundary::ffi_boundary;
    use crate::handles::decode_handle_for_context;
    use lumio_kernel::error::{ErrorCategory, MappingBlocked, to_architecture_error_code};
    use lumio_kernel::handle::{ContextKey, Generation, HandleKey, SlotIndex};

    fn wrong_context_key() -> HandleKey {
        HandleKey {
            context: ContextKey::new(1),
            slot: SlotIndex::new(0),
            generation: Generation::new(1),
        }
    }

    #[test]
    fn c_smoke_invalid_handle_returns_stable_code() {
        let key = wrong_context_key();
        let expected = ContextKey::new(2);

        let err = match smoke_decode_handle(key, expected) {
            Err(e) => e,
            Ok(()) => panic!("wrong-context smoke must fail"),
        };
        assert_eq!(err.category(), ErrorCategory::WrongContext);
        assert_ne!(err.category(), ErrorCategory::InvalidHandle);
        assert_ne!(err.category(), ErrorCategory::AlreadyReleased);
        assert_eq!(to_architecture_error_code(&err), Err(MappingBlocked));

        let via_boundary =
            match ffi_boundary(|| decode_handle_for_context(key, expected).map(|_| ())) {
                Err(e) => e,
                Ok(()) => panic!("wrong-context decode through ffi_boundary must fail"),
            };
        assert_eq!(via_boundary.category(), ErrorCategory::WrongContext);
        assert_eq!(via_boundary.category(), err.category());
        assert_eq!(
            to_architecture_error_code(&via_boundary),
            Err(MappingBlocked)
        );

        assert!(smoke_decode_handle(key, ContextKey::new(1)).is_ok());

        let exports_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/exports.rs");
        let exports_src = std::fs::read_to_string(&exports_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", exports_path.display()));
        assert!(
            !exports_src.contains(concat!("lumio_core_get_api_", "v1")),
            "exports.rs must not mention the Root API symbol"
        );
    }
}

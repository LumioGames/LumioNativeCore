//! T-ffi-04 / R-00179: blocked-header handle smoke returns WrongContext.
//!
//! `lumio-native-ffi` is `cdylib`+`staticlib` only, so Cargo does not pass
//! `--extern lumio_native_ffi` (no rlib) to integration tests on this host.
//! The smoke helper composes `ffi_boundary` and `decode_handle_for_context`,
//! so those sources are compiled via `#[path]` together with `exports.rs`.
//! The crate-root `pub use` is covered by the `#[cfg(test)]` module in
//! `exports.rs`.

#[path = "../src/boundary.rs"]
mod boundary;
#[path = "../src/exports.rs"]
mod exports;
#[path = "../src/handles.rs"]
mod handles;

use boundary::ffi_boundary;
use exports::smoke_decode_handle;
use handles::decode_handle_for_context;
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

    let via_boundary = match ffi_boundary(|| decode_handle_for_context(key, expected).map(|_| ())) {
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
        !exports_src.contains("lumio_core_get_api_v1"),
        "exports.rs must not mention the Root API symbol"
    );
}

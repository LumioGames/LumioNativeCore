//! Opaque handle decode for FFI: context check before registry lookup.
//!
//! Opaque width/bit layout and ContextId ABI representation are blocked
//! (B-ABI-003 / B-ABI-005, T-ffi-03). This helper only compares kernel
//! `HandleKey` against an expected `ContextKey`; it does not pack or unpack
//! a public ABI integer.

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};
use lumio_kernel::handle::{ContextKey, HandleKey};

/// Accept a kernel handle only when it belongs to `expected`.
///
/// Wrong context is `ErrorCategory::WrongContext` before InvalidHandle /
/// AlreadyReleased. Callers may then ask `TypedHandleRegistry` with the
/// returned key; the registry repeats the same check.
pub fn decode_handle_for_context(key: HandleKey, expected: ContextKey) -> KernelResult<HandleKey> {
    if key.context != expected {
        return Err(KernelError::new(
            ErrorCategory::WrongContext,
            ErrorDetail::None,
        ));
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::decode_handle_for_context;
    use lumio_kernel::error::ErrorCategory;
    use lumio_kernel::handle::{ContextKey, Generation, Handle, HandleKey, TypedHandleRegistry};

    #[test]
    fn wrong_context_opaque_handle_is_rejected() {
        let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 4);
        let live_key = registry.insert(10u32).expect("insert into context 1").key();
        assert_eq!(live_key.context, ContextKey::new(1));

        let decoded =
            decode_handle_for_context(live_key, ContextKey::new(1)).expect("same-context decode");
        assert_eq!(decoded, live_key);
        assert_eq!(
            *registry
                .get(Handle::from_key(decoded))
                .expect("live handle is visible after decode"),
            10
        );

        let err = match decode_handle_for_context(live_key, ContextKey::new(2)) {
            Err(e) => e,
            Ok(_) => panic!("wrong context decode must fail"),
        };
        assert_eq!(err.category(), ErrorCategory::WrongContext);
        assert_ne!(err.category(), ErrorCategory::InvalidHandle);
        assert_ne!(err.category(), ErrorCategory::AlreadyReleased);

        let wrong_key = HandleKey {
            context: ContextKey::new(2),
            slot: live_key.slot,
            generation: live_key.generation,
        };
        let stamped = match decode_handle_for_context(wrong_key, ContextKey::new(1)) {
            Err(e) => e,
            Ok(_) => panic!("stamped wrong-context key must fail"),
        };
        assert_eq!(stamped.category(), ErrorCategory::WrongContext);
        assert_ne!(stamped.category(), ErrorCategory::InvalidHandle);

        let wrong_gen_key = HandleKey {
            context: ContextKey::new(2),
            slot: live_key.slot,
            generation: Generation::new(live_key.generation.raw().wrapping_add(1)),
        };
        let stale = match decode_handle_for_context(wrong_gen_key, ContextKey::new(1)) {
            Err(e) => e,
            Ok(_) => panic!("wrong context must win over wrong generation"),
        };
        assert_eq!(stale.category(), ErrorCategory::WrongContext);
        assert_ne!(stale.category(), ErrorCategory::InvalidHandle);

        assert_eq!(
            *registry
                .get(Handle::from_key(live_key))
                .expect("live handle remains after wrong-context reject"),
            10
        );
    }
}

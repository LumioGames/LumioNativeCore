//! T-handle-02 / R-00088: bounded HandleArena insert rejects capacity exhaustion.

use lumio_kernel::error::{ErrorCategory, ErrorDetail};
use lumio_kernel::handle::{ContextKey, HandleArena};

#[test]
fn arena_rejects_capacity_exhaustion() {
    let mut arena = HandleArena::with_capacity(ContextKey::new(1), 1);

    assert!(arena.insert(10u32).is_ok());
    assert_eq!(arena.len(), 1);
    assert_eq!(arena.capacity(), 1);

    let err = match arena.insert(20u32) {
        Err(e) => e,
        Ok(_) => panic!("second insert must exhaust capacity"),
    };
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
    match err.detail() {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 1);
            assert_eq!(*requested, 2);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_eq!(arena.len(), 1);
}

//! T-handle-03 / R-00089: generation overflow permanently retires a HandleArena slot.

use lumio_kernel::error::{ErrorCategory, ErrorDetail};
use lumio_kernel::handle::{ContextKey, Generation, Handle, HandleArena};

#[test]
fn generation_overflow_retires_slot_permanently() {
    let mut arena = HandleArena::with_capacity(ContextKey::new(1), 1);
    let first = arena.insert(10u32).expect("insert into empty arena");
    let first_key = first.key();
    let first = Handle::from_key(first_key);
    assert_eq!(first_key.generation, Generation::new(1));
    assert_eq!(arena.remove(first).expect("remove live handle"), 10);
    assert_eq!(arena.len(), 0);

    let second = arena.insert(20u32).expect("reuse after remove");
    let second_key = second.key();
    assert_eq!(second_key.slot, first_key.slot);
    assert_eq!(second_key.generation, Generation::new(2));
    assert_eq!(arena.len(), 1);

    let stale = match arena.remove(Handle::from_key(first_key)) {
        Err(e) => e,
        Ok(_) => panic!("stale generation must not remove the reused slot"),
    };
    assert_eq!(stale.category(), ErrorCategory::InvalidHandle);

    assert_eq!(
        arena
            .remove(Handle::from_key(second_key))
            .expect("remove reused handle"),
        20
    );
    let released = match arena.remove(Handle::from_key(first_key)) {
        Err(e) => e,
        Ok(_) => panic!("empty slot must not yield a value"),
    };
    assert_eq!(released.category(), ErrorCategory::AlreadyReleased);

    let mut arena = HandleArena::with_capacity(ContextKey::new(1), 1);
    arena.force_generation(0, Generation::new(u32::MAX));
    let live = arena.insert(30u32).expect("insert at max generation");
    let live_key = live.key();
    assert_eq!(live_key.generation, Generation::new(u32::MAX));
    assert_eq!(
        arena
            .remove(Handle::from_key(live_key))
            .expect("remove at max generation"),
        30
    );
    assert_eq!(arena.len(), 0);

    let err = match arena.insert(40u32) {
        Err(e) => e,
        Ok(_) => panic!("retired slot must not return to the free list"),
    };
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
    match err.detail() {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 1);
            assert_eq!(*requested, 2);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    assert_eq!(arena.len(), 0);
    assert_eq!(arena.capacity(), 1);
}

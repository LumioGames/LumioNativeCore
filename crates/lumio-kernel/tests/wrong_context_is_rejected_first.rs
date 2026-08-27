//! T-handle-04 / R-00090: TypedHandleRegistry rejects wrong context first.

use lumio_kernel::error::ErrorCategory;
use lumio_kernel::handle::{ContextKey, Generation, Handle, HandleKey, TypedHandleRegistry};

#[test]
fn wrong_context_is_rejected_first() {
    let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 4);
    assert_eq!(registry.context(), ContextKey::new(1));

    let live_key = registry.insert(10u32).expect("insert into context 1").key();
    assert_eq!(live_key.context, ContextKey::new(1));
    assert_eq!(
        *registry
            .get(Handle::from_key(live_key))
            .expect("live handle is visible"),
        10
    );

    let wrong_key = HandleKey {
        context: ContextKey::new(2),
        slot: live_key.slot,
        generation: live_key.generation,
    };
    let get_err = match registry.get(Handle::from_key(wrong_key)) {
        Err(e) => e,
        Ok(_) => panic!("wrong context get must fail"),
    };
    assert_eq!(get_err.category(), ErrorCategory::WrongContext);
    assert_ne!(get_err.category(), ErrorCategory::InvalidHandle);
    assert_ne!(get_err.category(), ErrorCategory::AlreadyReleased);

    let wrong_gen_key = HandleKey {
        context: ContextKey::new(2),
        slot: live_key.slot,
        generation: Generation::new(live_key.generation.raw().wrapping_add(1)),
    };
    let get_stale = match registry.get(Handle::from_key(wrong_gen_key)) {
        Err(e) => e,
        Ok(_) => panic!("wrong context must win over wrong generation"),
    };
    assert_eq!(get_stale.category(), ErrorCategory::WrongContext);
    assert_ne!(get_stale.category(), ErrorCategory::InvalidHandle);

    let remove_err = match registry.remove(Handle::from_key(wrong_key)) {
        Err(e) => e,
        Ok(_) => panic!("wrong context remove must fail"),
    };
    assert_eq!(remove_err.category(), ErrorCategory::WrongContext);
    assert_ne!(remove_err.category(), ErrorCategory::InvalidHandle);
    assert_ne!(remove_err.category(), ErrorCategory::AlreadyReleased);

    assert_eq!(
        *registry
            .get(Handle::from_key(live_key))
            .expect("live handle remains after wrong-context reject"),
        10
    );
    assert_eq!(
        registry
            .remove(Handle::from_key(live_key))
            .expect("remove live handle"),
        10
    );
}

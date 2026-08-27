//! T-handle-05 / R-00129: borrow vs remove has a single winner.

use std::thread;

use lumio_kernel::error::ErrorCategory;
use lumio_kernel::handle::{ContextKey, TypedHandleRegistry};
use lumio_test_support::Interleaving;

#[test]
fn remove_vs_borrow_has_single_winner() {
    // Linearization 1: borrow holds the live value; remove proceeds after the guard drops.
    let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 4);
    let handle = registry.insert(10u32).expect("insert live handle");
    assert_eq!(registry.len(), 1);

    let interleaving = Interleaving::new(&["borrow", "remove"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("borrow");
            let guard = registry.borrow(handle).expect("borrow live handle");
            assert_eq!(*guard, 10);
            interleaving.arrive_and_wait("remove");
            drop(guard);
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("borrow");
            interleaving.arrive_and_wait("remove");
            assert_eq!(
                registry.remove(handle).expect("remove after borrow drops"),
                10
            );
        });
    });
    assert_eq!(registry.len(), 0);
    let after_remove = match registry.borrow(handle) {
        Err(err) => err,
        Ok(_) => panic!("borrow after remove must fail"),
    };
    assert_eq!(after_remove.category(), ErrorCategory::AlreadyReleased);

    // Linearization 2: remove wins first; later borrow observes AlreadyReleased.
    let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 4);
    let handle = registry.insert(20u32).expect("insert live handle");
    assert_eq!(registry.len(), 1);

    let interleaving = Interleaving::new(&["remove", "borrow"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("remove");
            assert_eq!(registry.remove(handle).expect("remove live handle"), 20);
            interleaving.arrive_and_wait("borrow");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("remove");
            interleaving.arrive_and_wait("borrow");
            let err = match registry.borrow(handle) {
                Err(err) => err,
                Ok(_) => panic!("borrow after remove must fail"),
            };
            assert_eq!(err.category(), ErrorCategory::AlreadyReleased);
        });
    });
    assert_eq!(registry.len(), 0);
}

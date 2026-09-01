//! ABI invalidCase `generation_overflow_is_fatal`.

mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn generation_overflow_is_fatal() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule");
    let maxed = manager.force_timer_generation(handle, u32::MAX);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let _ = manager.cancel(maxed);
    }));
    assert!(
        panicked.is_err(),
        "generation overflow must fail-stop, not wrap"
    );
}

#[test]
fn scope_generation_overflow_is_fatal() {
    let (mut manager, scope, _slot) = common::manager_at_tick(10);
    manager.force_scope_generation(scope.scope_id, u32::MAX);
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        let _ = manager.teardown_scope(scope.scope_id);
    }));
    assert!(panicked.is_err());
}

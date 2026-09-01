//! ABI invalidCases: stale handle, scope reset, generation mismatch, unregistered scope.

mod common;

use lumio_timer::{ScopeKind, TimerError, TimerScope};

#[test]
fn stale_handle_never_fuzzy_matches() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let first = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule H1");
    assert_eq!(manager.cancel(first), Ok(true));
    let second = manager
        .schedule_one_shot(scope, 16, slot)
        .expect("reuse index as H2");
    assert_eq!(first.index(), second.index());
    assert_ne!(first.generation(), second.generation());
    assert_ne!(first, second);
    assert_eq!(manager.cancel(first), Err(TimerError::StaleHandle));
    assert_eq!(manager.cancel(second), Ok(true));
}

#[test]
fn scope_reset_invalidates_all_handles() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule");
    let next = manager.teardown_scope(scope.scope_id).expect("teardown");
    assert_eq!(next.generation, 2);
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
    let records = manager.advance(15).expect("advance after teardown");
    assert!(records.firings().is_empty());
    let rebound = manager
        .schedule_one_shot(next, 16, slot)
        .expect("new generation schedules");
    assert_eq!(manager.cancel(rebound), Ok(true));
}

#[test]
fn scope_generation_mismatch_at_schedule() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let _ = manager.teardown_scope(scope.scope_id).expect("teardown");
    let err = manager
        .schedule_one_shot(scope, 15, slot)
        .expect_err("old generation must not schedule");
    assert_eq!(err, TimerError::ScopeGenerationMismatch);
    assert_eq!(err.as_str(), "scope_generation_mismatch");
}

#[test]
fn scope_invalid_unregistered_scope() {
    let (mut manager, _scope, slot) = common::manager_at_tick(10);
    let ghost = TimerScope {
        scope_id: 99,
        kind: ScopeKind::World,
        generation: 1,
    };
    let err = manager
        .schedule_one_shot(ghost, 15, slot)
        .expect_err("unregistered scope");
    assert_eq!(err, TimerError::ScopeInvalid);
    assert_eq!(err.as_str(), "scope_invalid");
}

#[test]
fn scope_invalid_unknown_kind() {
    let mut manager = lumio_timer::TimerManager::new(1);
    let err = manager
        .register_scope_from_u8(9, 1)
        .expect_err("unknown kind");
    assert_eq!(err, TimerError::ScopeInvalid);
}

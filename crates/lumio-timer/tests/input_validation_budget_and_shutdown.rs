//! ABI input validation, budget, and manager shutdown cases.

mod common;

use lumio_timer::{MAX_ACTIVE_TIMERS_PER_SCOPE, MAX_SCHEDULES_PER_TICK, TimerError};

#[test]
fn invalid_interval_rejected() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let err = manager
        .schedule_repeating(scope, 15, 0, slot)
        .expect_err("interval 0");
    assert_eq!(err, TimerError::InvalidInterval);
    assert_eq!(err.as_str(), "invalid_interval");
}

#[test]
fn invalid_due_tick_at_schedule() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let err = manager
        .schedule_one_shot(scope, 10, slot)
        .expect_err("due == committed");
    assert_eq!(err, TimerError::InvalidDueTick);
    assert_eq!(manager.committed_tick(), 10);
}

#[test]
fn advance_rollback_rejected() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 25, slot)
        .expect("schedule");
    let _ = manager.advance(20).expect("forward");
    let err = manager.advance(15).expect_err("rollback");
    assert_eq!(err, TimerError::InvalidDueTick);
    assert_eq!(manager.committed_tick(), 20);
    assert_eq!(manager.cancel(handle), Ok(true));
}

#[test]
fn schedule_budget_exceeded_no_partial_schedule() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let mut live = Vec::new();
    for i in 0..MAX_ACTIVE_TIMERS_PER_SCOPE {
        let handle = manager
            .schedule_one_shot(scope, 20 + u64::from(i), slot)
            .expect("under budget");
        live.push(handle);
    }
    let err = manager
        .schedule_one_shot(scope, 20 + u64::from(MAX_ACTIVE_TIMERS_PER_SCOPE), slot)
        .expect_err("active cap");
    assert_eq!(err, TimerError::ScheduleBudgetExceeded);
    assert_eq!(live.len() as u32, MAX_ACTIVE_TIMERS_PER_SCOPE);
}

#[test]
fn max_schedules_per_tick_exceeded() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    for _ in 0..MAX_SCHEDULES_PER_TICK {
        let handle = manager
            .schedule_one_shot(scope, 15, slot)
            .expect("schedule counted");
        assert_eq!(manager.cancel(handle), Ok(true));
    }
    let err = manager
        .schedule_one_shot(scope, 15, slot)
        .expect_err("tick schedule cap");
    assert_eq!(err, TimerError::ScheduleBudgetExceeded);
}

#[test]
fn manager_shutdown_rejects_all_operations() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule before shutdown");
    manager.shutdown();
    assert_eq!(
        manager.schedule_one_shot(scope, 16, slot),
        Err(TimerError::ManagerShutdown)
    );
    assert_eq!(
        manager.schedule_repeating(scope, 16, 2, slot),
        Err(TimerError::ManagerShutdown)
    );
    assert_eq!(manager.cancel(handle), Err(TimerError::ManagerShutdown));
    assert!(manager.advance(20).is_err());
    assert!(manager.advance(5).is_err());
    assert_eq!(
        manager.cancel(handle).unwrap_err().as_str(),
        "manager_shutdown"
    );
    assert_eq!(manager.drain_records(), Err(TimerError::ManagerShutdown));
    assert_eq!(manager.pump(30), Err(TimerError::ManagerShutdown));
}

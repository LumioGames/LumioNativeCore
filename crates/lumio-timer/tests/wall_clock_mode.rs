//! C-4′ wallClock mode: pump(now_ms), cancel, stale handle, isolated handle space.

mod common;

use lumio_timer::{CallbackSlot, MIN_INTERVAL_MS, ScopeKind, TimerError, TimerManager, TimerMode};

fn wall_clock_at_ms(ms: u64) -> (TimerManager, lumio_timer::TimerScope, CallbackSlot) {
    let mut manager = TimerManager::with_mode(7, TimerMode::WallClock);
    assert_eq!(manager.mode(), TimerMode::WallClock);
    let scope = manager
        .register_scope(1, ScopeKind::Session)
        .expect("register scope");
    let slot = manager.create_slot().expect("create slot");
    manager
        .bind_slot(slot, common::TEST_DISPATCH)
        .expect("bind slot");
    if ms > 0 {
        let report = manager.pump(ms).expect("prime committed ms");
        assert!(report.firings().is_empty(), "prime pump must be empty");
    }
    (manager, scope, slot)
}

#[test]
fn wall_clock_one_shot_fires_exactly_once_on_pump() {
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let handle = manager
        .schedule_one_shot(scope, 1500, slot)
        .expect("schedule wallClock one-shot");

    let at_due = manager.pump(1500).expect("pump to due");
    assert_eq!(common::dues(at_due.firings()), vec![1500]);
    assert_eq!(at_due.firings()[0].handle, handle);
    assert_eq!(
        at_due.firings()[0].slot_dispatch_id.raw(),
        common::TEST_DISPATCH.raw()
    );

    let delivered = manager.drain_records().expect("drain");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].due_tick, 1500);
    assert_eq!(delivered[0].handle, handle);

    let after = manager.pump(1501).expect("pump past due");
    assert!(
        after.firings().iter().all(|r| r.handle != handle),
        "one-shot must not fire again"
    );
    assert!(manager.drain_records().expect("second drain").is_empty());
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
}

#[test]
fn wall_clock_cancel_prevents_delivery() {
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let handle = manager
        .schedule_one_shot(scope, 1500, slot)
        .expect("schedule");
    assert_eq!(manager.cancel(handle), Ok(true));
    let fired = manager.pump(1500).expect("pump after cancel");
    assert!(fired.firings().is_empty());
    assert!(manager.drain_records().expect("drain").is_empty());
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
}

#[test]
fn wall_clock_stale_handle_never_fuzzy_matches() {
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let first = manager.schedule_one_shot(scope, 1500, slot).expect("H1");
    assert_eq!(manager.cancel(first), Ok(true));
    let second = manager
        .schedule_one_shot(scope, 1600, slot)
        .expect("H2 reuses index");
    assert_eq!(first.index(), second.index());
    assert_ne!(first.generation(), second.generation());
    assert_eq!(manager.cancel(first), Err(TimerError::StaleHandle));
    assert_eq!(manager.cancel(second), Ok(true));
}

#[test]
fn wall_clock_pump_rollback_rejected() {
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let handle = manager
        .schedule_one_shot(scope, 2500, slot)
        .expect("schedule");
    let _ = manager.pump(2000).expect("forward");
    let err = manager.pump(1500).expect_err("rollback");
    assert_eq!(err, TimerError::InvalidDueTick);
    assert_eq!(manager.committed_tick(), 2000);
    assert_eq!(manager.cancel(handle), Ok(true));
}

#[test]
fn wall_clock_repeating_uses_interval_ms() {
    assert_eq!(MIN_INTERVAL_MS, 1);
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let handle = manager
        .schedule_repeating(scope, 1200, 500, slot)
        .expect("repeating");
    let records = manager.pump(2200).expect("pump window");
    assert_eq!(common::dues(records.firings()), vec![1200, 1700, 2200]);
    assert!(records.firings().iter().all(|r| r.handle == handle));
}

#[test]
fn wall_clock_and_tick_frame_handle_spaces_do_not_alias() {
    let (mut wall, wall_scope, wall_slot) = wall_clock_at_ms(1000);
    let (mut ticks, tick_scope, tick_slot) = common::manager_at_tick(10);
    assert_eq!(ticks.mode(), TimerMode::TickFrame);

    let wall_handle = wall
        .schedule_one_shot(wall_scope, 1500, wall_slot)
        .expect("wall handle");
    let tick_handle = ticks
        .schedule_one_shot(tick_scope, 15, tick_slot)
        .expect("tick handle");

    assert_eq!(wall.cancel(tick_handle), Err(TimerError::StaleHandle));
    assert_eq!(ticks.cancel(wall_handle), Err(TimerError::StaleHandle));
    assert_eq!(wall.cancel(wall_handle), Ok(true));
    assert_eq!(ticks.cancel(tick_handle), Ok(true));
}

#[test]
fn wall_clock_invalid_interval_ms_rejected() {
    let (mut manager, scope, slot) = wall_clock_at_ms(1000);
    let err = manager
        .schedule_repeating(scope, 1500, 0, slot)
        .expect_err("interval 0");
    assert_eq!(err, TimerError::InvalidInterval);
}

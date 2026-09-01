//! ABI CallbackSlot lifecycle and failure paths — per-case asserts.

mod common;

use lumio_timer::{DELIVERY_QUEUE_DEPTH_PER_SLOT, SlotDispatchId, TimerError, TimerManager};

#[test]
fn late_completion_terminal_no_state_write() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule");
    let records = manager.advance(15).expect("advance produces record");
    assert_eq!(records.firings().len(), 1);
    assert_eq!(manager.cancel(handle), Ok(true));
    let drained = manager.drain();
    assert!(drained.delivered().is_empty());
    assert_eq!(drained.rejections().len(), 1);
    assert_eq!(drained.rejections()[0].code, TimerError::LateCompletion);
    assert_eq!(drained.rejections()[0].code.as_str(), "late_completion");
}

#[test]
fn slot_failure_explicit_not_silent() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule");
    manager.close_slot(slot).expect("close before due");
    let records = manager.advance(15).expect("advance to due");
    assert!(
        records.firings().is_empty(),
        "closed slot must not produce FiringRecord"
    );
    assert!(
        records
            .rejections()
            .iter()
            .any(|r| r.code == TimerError::SlotClosed)
    );
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
    assert!(manager.drain().delivered().is_empty());
}

#[test]
fn slot_unbound_rejected_at_schedule() {
    let mut manager = TimerManager::new(1);
    let scope = manager
        .register_scope(1, lumio_timer::ScopeKind::World)
        .expect("scope");
    let unbound = manager.create_slot().expect("unbound slot");
    let err = manager
        .schedule_one_shot(scope, 15, unbound)
        .expect_err("unbound");
    assert_eq!(err, TimerError::SlotUnbound);
    assert_eq!(err.as_str(), "slot_unbound");
}

#[test]
fn slot_closed_rejected_at_schedule() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    manager.close_slot(slot).expect("close armed slot");
    let err = manager
        .schedule_one_shot(scope, 15, slot)
        .expect_err("closed");
    assert_eq!(err, TimerError::SlotClosed);
    assert_eq!(err.as_str(), "slot_closed");
    assert_ne!(err, TimerError::SlotUnbound);
}

#[test]
fn slot_queue_full_timer_terminal_process_lives() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_repeating(scope, 11, 1, slot)
        .expect("repeating");
    let first = manager
        .advance(10 + DELIVERY_QUEUE_DEPTH_PER_SLOT as u64)
        .expect("fill window");
    assert_eq!(first.firings().len(), DELIVERY_QUEUE_DEPTH_PER_SLOT);
    assert!(first.rejections().is_empty());
    let extra = manager
        .advance(11 + DELIVERY_QUEUE_DEPTH_PER_SLOT as u64)
        .expect("manager continues");
    assert!(extra.firings().is_empty());
    assert_eq!(extra.rejections().len(), 1);
    assert_eq!(extra.rejections()[0].code, TimerError::SlotQueueFull);
    assert_eq!(extra.rejections()[0].code.as_str(), "slot_queue_full");
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
    let other_slot = manager.create_slot().expect("other slot");
    manager
        .bind_slot(other_slot, SlotDispatchId::from_static("test.other"))
        .expect("bind other");
    let other = manager
        .schedule_one_shot(scope, manager.committed_tick() + 2, other_slot)
        .expect("manager still schedules");
    let later = manager
        .advance(manager.committed_tick() + 2)
        .expect("advance other");
    assert_eq!(later.firings().len(), 1);
    assert_eq!(later.firings()[0].handle, other);
}

#[test]
fn slot_dispatch_mismatch_at_drain() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule");
    let records = manager.advance(15).expect("advance");
    assert_eq!(records.firings().len(), 1);
    manager.remove_dispatch_binding(common::TEST_DISPATCH);
    let drained = manager.drain();
    assert!(drained.delivered().is_empty());
    assert_eq!(
        drained.rejections()[0].code,
        TimerError::SlotDispatchMismatch
    );
    assert_eq!(
        drained.rejections()[0].code.as_str(),
        "slot_dispatch_mismatch"
    );
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
}

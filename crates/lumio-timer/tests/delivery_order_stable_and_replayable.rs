//! ABI testCase `delivery_order_stable_and_replayable`.

mod common;

use lumio_timer::{ScopeKind, TimerManager};

fn schedule_pair(
    manager: &mut TimerManager,
) -> (lumio_timer::TimerHandle, lumio_timer::TimerHandle) {
    let scope = manager
        .register_scope(1, ScopeKind::World)
        .expect("register");
    let slot_a = manager.create_slot().expect("slot A");
    manager
        .bind_slot(slot_a, common::TEST_DISPATCH)
        .expect("bind A");
    let slot_b = manager.create_slot().expect("slot B");
    manager
        .bind_slot(slot_b, common::TEST_DISPATCH_B)
        .expect("bind B");
    let a = manager
        .schedule_one_shot(scope, 20, slot_a)
        .expect("schedule A");
    let b = manager
        .schedule_one_shot(scope, 20, slot_b)
        .expect("schedule B");
    (a, b)
}

#[test]
fn delivery_order_stable_and_replayable() {
    let mut first = TimerManager::new(1);
    let _ = first.advance(10).expect("prime");
    let (a1, b1) = schedule_pair(&mut first);
    let records_1 = first.advance(20).expect("advance 1");
    assert_eq!(records_1.firings().len(), 2);
    assert_eq!(records_1.firings()[0].handle, a1);
    assert_eq!(records_1.firings()[1].handle, b1);
    assert!(records_1.firings()[0].schedule_sequence < records_1.firings()[1].schedule_sequence);

    let mut second = TimerManager::new(1);
    let _ = second.advance(10).expect("prime");
    let (a2, b2) = schedule_pair(&mut second);
    let records_2 = second.advance(20).expect("advance 2");
    assert_eq!(records_2.firings()[0].handle, a2);
    assert_eq!(records_2.firings()[1].handle, b2);
    assert_eq!(
        records_1
            .firings()
            .iter()
            .map(|r| (r.due_tick, r.schedule_sequence))
            .collect::<Vec<_>>(),
        records_2
            .firings()
            .iter()
            .map(|r| (r.due_tick, r.schedule_sequence))
            .collect::<Vec<_>>()
    );
}

#[test]
fn drain_records_follow_due_and_sequence_not_slot_index() {
    let mut manager = TimerManager::new(1);
    let scope = manager
        .register_scope(1, ScopeKind::World)
        .expect("register");
    let slot_a = manager.create_slot().expect("slot A index 0");
    manager
        .bind_slot(slot_a, common::TEST_DISPATCH)
        .expect("bind A");
    let slot_b = manager.create_slot().expect("slot B index 1");
    manager
        .bind_slot(slot_b, common::TEST_DISPATCH_B)
        .expect("bind B");
    assert!(
        slot_a.index() < slot_b.index(),
        "A must be the earlier slot"
    );

    let handle_b = manager
        .schedule_one_shot(scope, 5, slot_b)
        .expect("B due=5 first, sequence 1");
    let handle_a = manager
        .schedule_one_shot(scope, 10, slot_a)
        .expect("A due=10 second, sequence 2");

    let firings = manager.advance(10).expect("advance covers both");
    assert_eq!(
        firings
            .firings()
            .iter()
            .map(|r| (r.due_tick, r.handle))
            .collect::<Vec<_>>(),
        vec![(5, handle_b), (10, handle_a)],
        "advance return set is (dueTick, scheduleSequence)"
    );

    let drained = manager.drain_records().expect("drain");
    assert_eq!(
        drained
            .iter()
            .map(|r| (r.due_tick, r.handle, r.schedule_sequence))
            .collect::<Vec<_>>(),
        firings
            .firings()
            .iter()
            .map(|r| (r.due_tick, r.handle, r.schedule_sequence))
            .collect::<Vec<_>>(),
        "drain must match advance order, not slot index (A=0 would otherwise precede B=1)"
    );
    assert_eq!(drained[0].handle, handle_b);
    assert_eq!(drained[1].handle, handle_a);
}

#[test]
fn drain_records_same_due_follow_schedule_sequence_not_slot_index() {
    let mut manager = TimerManager::new(1);
    let _ = manager.advance(10).expect("prime");
    let (handle_a, handle_b) = schedule_pair(&mut manager);
    let firings = manager.advance(20).expect("advance");
    assert_eq!(firings.firings()[0].handle, handle_a);
    assert_eq!(firings.firings()[1].handle, handle_b);

    let mut reversed = TimerManager::new(1);
    let _ = reversed.advance(10).expect("prime");
    let scope = reversed
        .register_scope(1, ScopeKind::World)
        .expect("register");
    let slot_a = reversed.create_slot().expect("A");
    reversed
        .bind_slot(slot_a, common::TEST_DISPATCH)
        .expect("bind A");
    let slot_b = reversed.create_slot().expect("B");
    reversed
        .bind_slot(slot_b, common::TEST_DISPATCH_B)
        .expect("bind B");
    let first_b = reversed
        .schedule_one_shot(scope, 20, slot_b)
        .expect("schedule B first");
    let second_a = reversed
        .schedule_one_shot(scope, 20, slot_a)
        .expect("schedule A second");
    let firings = reversed.advance(20).expect("advance reversed");
    assert_eq!(firings.firings()[0].handle, first_b);
    assert_eq!(firings.firings()[1].handle, second_a);
    let drained = reversed.drain_records().expect("drain");
    assert_eq!(drained[0].handle, first_b);
    assert_eq!(drained[1].handle, second_a);
    assert!(drained[0].schedule_sequence < drained[1].schedule_sequence);
}

#[test]
fn drain_after_two_advances_without_intermediate_drain_keeps_global_order() {
    let mut manager = TimerManager::new(1);
    let scope = manager
        .register_scope(1, ScopeKind::World)
        .expect("register");
    let slot_a = manager.create_slot().expect("A");
    manager
        .bind_slot(slot_a, common::TEST_DISPATCH)
        .expect("bind A");
    let slot_b = manager.create_slot().expect("B");
    manager
        .bind_slot(slot_b, common::TEST_DISPATCH_B)
        .expect("bind B");

    let handle_b = manager
        .schedule_one_shot(scope, 5, slot_b)
        .expect("B due=5");
    let handle_a = manager
        .schedule_one_shot(scope, 10, slot_a)
        .expect("A due=10");
    let _ = manager.advance(5).expect("first window");
    let _ = manager.advance(10).expect("second window, no drain yet");
    let drained = manager.drain_records().expect("one drain");
    assert_eq!(
        drained.iter().map(|r| r.handle).collect::<Vec<_>>(),
        vec![handle_b, handle_a],
        "later window on lower slot index must not precede earlier window"
    );
}

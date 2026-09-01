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

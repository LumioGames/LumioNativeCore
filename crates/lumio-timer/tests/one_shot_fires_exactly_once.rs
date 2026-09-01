//! ABI testCase `one_shot_fires_exactly_once`.

mod common;

use lumio_timer::TimerError;

#[test]
fn one_shot_fires_exactly_once() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule one-shot");

    let at_due = manager.advance(15).expect("advance to due");
    assert_eq!(common::dues(at_due.firings()), vec![15]);
    assert_eq!(at_due.firings().len(), 1);
    assert_eq!(at_due.firings()[0].handle, handle);
    assert_eq!(at_due.firings()[0].due_tick, 15);
    let seq = at_due.firings()[0].schedule_sequence;

    let delivered = manager.drain();
    assert_eq!(delivered.delivered().len(), 1);
    assert_eq!(delivered.delivered()[0].due_tick, 15);
    assert_eq!(delivered.delivered()[0].handle, handle);

    let after = manager.advance(16).expect("advance past due");
    assert!(
        after.firings().iter().all(|r| r.handle != handle),
        "one-shot must not fire again"
    );
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
    assert!(seq >= 1);
}

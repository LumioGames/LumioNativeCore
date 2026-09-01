//! ABI testCase `cancel_prevents_delivery` and invalidCase `double_cancel_stable_error`.

mod common;

use lumio_timer::TimerError;

#[test]
fn cancel_prevents_delivery() {
    let (mut manager, scope, slot) = common::manager_at_tick(12);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule one-shot");
    assert_eq!(manager.cancel(handle), Ok(true));

    let records = manager.advance(15).expect("advance after cancel");
    assert!(
        records.firings().is_empty(),
        "cancelled timer must not fire"
    );
    assert!(manager.drain().delivered().is_empty());
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
}

#[test]
fn double_cancel_stable_error() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_one_shot(scope, 15, slot)
        .expect("schedule one-shot");
    assert_eq!(manager.cancel(handle), Ok(true));
    assert_eq!(manager.cancel(handle), Err(TimerError::StaleHandle));
    let records = manager.advance(15).expect("advance");
    assert!(records.firings().is_empty());
}

//! ABI testCase `repeating_fires_each_interval`.

mod common;

#[test]
fn repeating_fires_each_interval() {
    let (mut manager, scope, slot) = common::manager_at_tick(10);
    let handle = manager
        .schedule_repeating(scope, 12, 5, slot)
        .expect("schedule repeating");

    let records = manager.advance(22).expect("advance catch-up");
    assert_eq!(
        common::dues(records.firings()),
        vec![12, 17, 22],
        "window (10, 22] must fire 12, 17, and 22"
    );
    assert!(records.firings().iter().all(|r| r.handle == handle));
    assert_eq!(records.firings().len(), 3);
    let sequences: Vec<u64> = records
        .firings()
        .iter()
        .map(|r| r.schedule_sequence)
        .collect();
    assert!(sequences.windows(2).all(|w| w[0] == w[1]));

    let delivered = manager.drain();
    assert_eq!(
        delivered
            .delivered()
            .iter()
            .map(|d| d.due_tick)
            .collect::<Vec<_>>(),
        vec![12, 17, 22]
    );
}

//! T-test-support-02: named Interleaving is a replayable barrier.

use std::sync::Mutex;
use std::thread;

use lumio_test_support::Interleaving;

fn run_named_pair() -> Vec<&'static str> {
    let events = Mutex::new(Vec::<&'static str>::new());
    let interleaving = Interleaving::new(&["a", "b"]);
    thread::scope(|scope| {
        for _ in 0..2 {
            scope.spawn(|| {
                interleaving.arrive_and_wait("a");
                events.lock().expect("events mutex").push("a");
                interleaving.arrive_and_wait("b");
                events.lock().expect("events mutex").push("b");
            });
        }
    });
    events.into_inner().expect("events mutex")
}

fn assert_both_a_before_any_b(events: &[&str]) {
    let first_b = events
        .iter()
        .position(|step| *step == "b")
        .expect("step b must be recorded");
    assert_eq!(
        &events[..first_b],
        ["a", "a"],
        "both threads must pass a before either records b"
    );
    assert_eq!(events, &["a", "a", "b", "b"]);
}

#[test]
fn named_interleaving_is_replayable() {
    let first = run_named_pair();
    assert_both_a_before_any_b(&first);
    let second = run_named_pair();
    assert_both_a_before_any_b(&second);
    assert_eq!(first, second, "identical step sequences on replay");
}

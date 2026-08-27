//! T-context-02 / R-00130: ContextStateGate close vs admit has one linearization.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use lumio_kernel::context::{ContextPhase, ContextStateGate};
use lumio_kernel::error::ErrorCategory;
use lumio_test_support::Interleaving;

#[test]
fn close_vs_admit_has_one_linearization() {
    let running = ContextStateGate::new_running();
    assert_eq!(running.snapshot().phase, ContextPhase::Running);
    running.try_admit().expect("admit while running");

    assert!(running.begin_close(), "first close wins CAS to Quiescing");
    assert_eq!(running.snapshot().phase, ContextPhase::Quiescing);
    let closing = running
        .try_admit()
        .expect_err("admit after begin_close must fail");
    assert_eq!(closing.category(), ErrorCategory::ContextClosing);
    assert!(!running.begin_close(), "second close does not win CAS");

    running.mark_closed();
    assert_eq!(running.snapshot().phase, ContextPhase::Closed);
    let destroyed = running
        .try_admit()
        .expect_err("admit after closed must fail");
    assert_eq!(destroyed.category(), ErrorCategory::ContextDestroyed);

    // Linearization 1: close CAS first; later admit observes ContextClosing.
    let gate = ContextStateGate::new_running();
    let close_won_first = AtomicBool::new(false);
    let admit_ok_after_close = AtomicBool::new(false);
    let interleaving = Interleaving::new(&["close", "admit"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("close");
            assert!(gate.begin_close(), "close wins while Running");
            close_won_first.store(true, Ordering::SeqCst);
            interleaving.arrive_and_wait("admit");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("close");
            interleaving.arrive_and_wait("admit");
            match gate.try_admit() {
                Ok(()) => admit_ok_after_close.store(true, Ordering::SeqCst),
                Err(err) => assert_eq!(err.category(), ErrorCategory::ContextClosing),
            }
        });
    });
    assert!(close_won_first.load(Ordering::SeqCst));
    assert!(
        !admit_ok_after_close.load(Ordering::SeqCst),
        "admit after closed must not succeed"
    );
    assert_eq!(gate.snapshot().phase, ContextPhase::Quiescing);

    // Linearization 2: admit while still Running; close still succeeds.
    let gate = ContextStateGate::new_running();
    let admit_ok_while_running = AtomicBool::new(false);
    let close_after_admit = AtomicBool::new(false);
    let interleaving = Interleaving::new(&["admit", "close"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("admit");
            gate.try_admit().expect("admit while running");
            admit_ok_while_running.store(true, Ordering::SeqCst);
            interleaving.arrive_and_wait("close");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("admit");
            interleaving.arrive_and_wait("close");
            assert!(gate.begin_close(), "close succeeds after admit");
            close_after_admit.store(true, Ordering::SeqCst);
        });
    });
    assert!(admit_ok_while_running.load(Ordering::SeqCst));
    assert!(close_after_admit.load(Ordering::SeqCst));
    assert_eq!(gate.snapshot().phase, ContextPhase::Quiescing);
}

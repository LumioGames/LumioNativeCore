//! T-job-03 / R-00144: every legal JobStateMachine CAS has a single winner.

use std::thread;

use lumio_job::{JobState, JobStateCell, JobStateMachine};
use lumio_test_support::Interleaving;

fn queued_cell() -> JobStateCell {
    JobStateMachine::queued()
}

fn assert_start_vs_cancel_queued_winner(
    start: Result<(), JobState>,
    cancel: Result<JobState, JobState>,
    snapshot: JobState,
) {
    let start_took_queued = start.is_ok();
    let cancel_took_queued = cancel == Ok(JobState::Queued);
    assert_ne!(
        start_took_queued, cancel_took_queued,
        "exactly one CAS leaves Queued: start={start:?} cancel={cancel:?} snapshot={snapshot:?}"
    );
    match snapshot {
        JobState::Running => {
            assert_eq!(start, Ok(()));
            assert_eq!(cancel, Err(JobState::Running));
        }
        JobState::Cancelled => {
            if cancel_took_queued {
                assert_eq!(start, Err(JobState::Cancelled));
                assert_eq!(cancel, Ok(JobState::Queued));
            } else {
                assert_eq!(start, Ok(()));
                assert_eq!(cancel, Ok(JobState::Running));
            }
        }
        other => panic!("Queued start/cancel must end Running or Cancelled, got {other:?}"),
    }
}

fn assert_complete_vs_cancel_running_winner(
    complete: Result<(), JobState>,
    cancel: Result<JobState, JobState>,
    snapshot: JobState,
    to: JobState,
) {
    assert_ne!(
        complete.is_ok(),
        cancel.is_ok(),
        "exactly one CAS leaves Running: complete={complete:?} cancel={cancel:?} snapshot={snapshot:?}"
    );
    if complete.is_ok() {
        assert_eq!(snapshot, to);
        assert_eq!(cancel, Err(to));
    } else {
        assert_eq!(snapshot, JobState::Cancelled);
        assert_eq!(cancel, Ok(JobState::Running));
        assert_eq!(complete, Err(JobState::Cancelled));
    }
}

#[test]
fn every_legal_transition_has_single_winner() {
    sequential_legal_transitions();
    start_then_cancel_linearization();
    cancel_then_start_linearization();
    complete_then_cancel_linearization(JobState::Succeeded);
    complete_then_cancel_linearization(JobState::Failed);
    cancel_then_complete_linearization(JobState::Succeeded);
    cancel_then_complete_linearization(JobState::Failed);
    concurrent_start_vs_cancel_from_queued();
    concurrent_complete_vs_cancel_from_running(JobState::Succeeded);
    concurrent_complete_vs_cancel_from_running(JobState::Failed);
    concurrent_double_start();
    concurrent_double_complete();
}

fn sequential_legal_transitions() {
    let machine = queued_cell();
    assert_eq!(machine.snapshot(), JobState::Queued);
    assert_eq!(machine.cas_start(), Ok(()));
    assert_eq!(machine.snapshot(), JobState::Running);
    assert_eq!(machine.cas_complete(JobState::Succeeded), Ok(()));
    assert_eq!(machine.snapshot(), JobState::Succeeded);
    assert_eq!(machine.cas_start(), Err(JobState::Succeeded));
    assert_eq!(
        machine.cas_complete(JobState::Failed),
        Err(JobState::Succeeded)
    );
    assert_eq!(machine.cas_cancel(), Err(JobState::Succeeded));

    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    assert_eq!(machine.cas_complete(JobState::Failed), Ok(()));
    assert_eq!(machine.snapshot(), JobState::Failed);
    assert_eq!(machine.cas_cancel(), Err(JobState::Failed));

    let machine = queued_cell();
    assert_eq!(machine.cas_cancel(), Ok(JobState::Queued));
    assert_eq!(machine.snapshot(), JobState::Cancelled);
    assert_eq!(machine.cas_start(), Err(JobState::Cancelled));
    assert_eq!(machine.cas_cancel(), Err(JobState::Cancelled));
    assert_eq!(
        machine.cas_complete(JobState::Succeeded),
        Err(JobState::Cancelled)
    );

    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    assert_eq!(machine.cas_cancel(), Ok(JobState::Running));
    assert_eq!(machine.snapshot(), JobState::Cancelled);
    assert_eq!(machine.cas_start(), Err(JobState::Cancelled));

    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    assert_eq!(
        machine.cas_complete(JobState::TimedOut),
        Err(JobState::Running)
    );
    assert_eq!(
        machine.cas_complete(JobState::Queued),
        Err(JobState::Running)
    );
    assert_eq!(
        machine.cas_complete(JobState::Cancelled),
        Err(JobState::Running)
    );
    assert_eq!(machine.snapshot(), JobState::Running);
    assert_eq!(machine.cas_start(), Err(JobState::Running));
}

fn start_then_cancel_linearization() {
    let machine = queued_cell();
    let interleaving = Interleaving::new(&["start", "cancel"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("start");
            assert_eq!(machine.cas_start(), Ok(()));
            interleaving.arrive_and_wait("cancel");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("start");
            interleaving.arrive_and_wait("cancel");
            assert_eq!(machine.cas_cancel(), Ok(JobState::Running));
        });
    });
    assert_eq!(machine.snapshot(), JobState::Cancelled);
}

fn cancel_then_start_linearization() {
    let machine = queued_cell();
    let interleaving = Interleaving::new(&["cancel", "start"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("cancel");
            assert_eq!(machine.cas_cancel(), Ok(JobState::Queued));
            interleaving.arrive_and_wait("start");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("cancel");
            interleaving.arrive_and_wait("start");
            assert_eq!(machine.cas_start(), Err(JobState::Cancelled));
        });
    });
    assert_eq!(machine.snapshot(), JobState::Cancelled);
}

fn complete_then_cancel_linearization(to: JobState) {
    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    let interleaving = Interleaving::new(&["complete", "cancel"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("complete");
            assert_eq!(machine.cas_complete(to), Ok(()));
            interleaving.arrive_and_wait("cancel");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("complete");
            interleaving.arrive_and_wait("cancel");
            assert_eq!(machine.cas_cancel(), Err(to));
        });
    });
    assert_eq!(machine.snapshot(), to);
}

fn cancel_then_complete_linearization(to: JobState) {
    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    let interleaving = Interleaving::new(&["cancel", "complete"]);
    thread::scope(|scope| {
        scope.spawn(|| {
            interleaving.arrive_and_wait("cancel");
            assert_eq!(machine.cas_cancel(), Ok(JobState::Running));
            interleaving.arrive_and_wait("complete");
        });
        scope.spawn(|| {
            interleaving.arrive_and_wait("cancel");
            interleaving.arrive_and_wait("complete");
            assert_eq!(machine.cas_complete(to), Err(JobState::Cancelled));
        });
    });
    assert_eq!(machine.snapshot(), JobState::Cancelled);
}

fn concurrent_start_vs_cancel_from_queued() {
    let machine = queued_cell();
    let interleaving = Interleaving::new(&["go"]);
    let start = thread::scope(|scope| {
        let start = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_start()
        });
        let cancel = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_cancel()
        });
        (
            start.join().expect("start thread"),
            cancel.join().expect("cancel thread"),
        )
    });
    assert_start_vs_cancel_queued_winner(start.0, start.1, machine.snapshot());
}

fn concurrent_complete_vs_cancel_from_running(to: JobState) {
    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    let interleaving = Interleaving::new(&["go"]);
    let outcome = thread::scope(|scope| {
        let complete = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_complete(to)
        });
        let cancel = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_cancel()
        });
        (
            complete.join().expect("complete thread"),
            cancel.join().expect("cancel thread"),
        )
    });
    assert_complete_vs_cancel_running_winner(outcome.0, outcome.1, machine.snapshot(), to);
}

fn concurrent_double_start() {
    let machine = queued_cell();
    let interleaving = Interleaving::new(&["go"]);
    let (a, b) = thread::scope(|scope| {
        let a = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_start()
        });
        let b = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_start()
        });
        (a.join().expect("start a"), b.join().expect("start b"))
    });
    assert_ne!(a.is_ok(), b.is_ok(), "exactly one cas_start wins");
    assert_eq!(machine.snapshot(), JobState::Running);
    assert!(a == Ok(()) || b == Ok(()));
    assert!(a == Err(JobState::Running) || b == Err(JobState::Running));
}

fn concurrent_double_complete() {
    let machine = queued_cell();
    assert_eq!(machine.cas_start(), Ok(()));
    let interleaving = Interleaving::new(&["go"]);
    let (a, b) = thread::scope(|scope| {
        let a = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_complete(JobState::Succeeded)
        });
        let b = scope.spawn(|| {
            interleaving.arrive_and_wait("go");
            machine.cas_complete(JobState::Failed)
        });
        (
            a.join().expect("complete succeeded"),
            b.join().expect("complete failed"),
        )
    });
    assert_ne!(a.is_ok(), b.is_ok(), "exactly one cas_complete wins");
    match machine.snapshot() {
        JobState::Succeeded => {
            assert_eq!(a, Ok(()));
            assert_eq!(b, Err(JobState::Succeeded));
        }
        JobState::Failed => {
            assert_eq!(b, Ok(()));
            assert_eq!(a, Err(JobState::Failed));
        }
        other => panic!("complete/complete must end Succeeded or Failed, got {other:?}"),
    }
}

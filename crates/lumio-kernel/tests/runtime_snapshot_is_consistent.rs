//! T-capability-03 / R-00085: RuntimeCounters snapshot matches last writes and saturates at 0.

use lumio_kernel::capability::{RuntimeCounters, RuntimeStatus};

#[test]
fn runtime_snapshot_is_consistent() {
    let counters = RuntimeCounters::new();
    counters.set_accepting_work(false);
    counters.add_queued(3);
    counters.add_running(1);
    counters.add_bytes(100);

    assert_eq!(
        counters.snapshot(),
        RuntimeStatus {
            accepting_work: false,
            queued_jobs: 3,
            running_jobs: 1,
            allocated_native_bytes: 100,
        }
    );

    counters.add_queued(-3);
    assert_eq!(counters.snapshot().queued_jobs, 0);

    counters.add_queued(-1);
    assert_eq!(counters.snapshot().queued_jobs, 0);
}

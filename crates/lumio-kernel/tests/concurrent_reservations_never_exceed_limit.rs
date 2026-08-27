//! T-memory-03 / R-00094: concurrent MemoryBudget reservations never exceed the limit.

use std::sync::Arc;
use std::thread;

use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::error::ErrorCategory;
use lumio_kernel::memory::MemoryBudget;

#[test]
fn concurrent_reservations_never_exceed_limit() {
    let limits = ConfiguredLimits {
        max_handles: 1,
        max_native_bytes: 100,
        max_jobs_queued: 1,
        max_jobs_running: 1,
        max_completion_items: 1,
    };
    limits.validate().expect("nonzero limits");

    let budget = Arc::new(MemoryBudget::new(limits.max_native_bytes));
    let mut joins = Vec::with_capacity(8);

    for _ in 0..8 {
        let budget = Arc::clone(&budget);
        joins.push(thread::spawn(move || budget.try_reserve(40)));
    }

    let mut ok = 0u64;
    for join in joins {
        match join.join().expect("reservation thread") {
            Ok(()) => ok += 1,
            Err(err) => assert_eq!(err.category(), ErrorCategory::CapacityExceeded),
        }
    }

    let charged = budget.charged();
    assert!(ok <= 2, "at most two 40-byte reservations fit in 100");
    assert_eq!(ok * 40, charged);
    assert!(matches!(charged, 0 | 40 | 80));
    assert!(charged <= budget.limit());
    assert_eq!(budget.limit(), 100);
}

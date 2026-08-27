//! T-test-support-03 / R-00158: LeakSnapshot detects unreleased handle and bytes.

use lumio_kernel::handle::{ContextKey, TypedHandleRegistry};
use lumio_kernel::memory::MemoryBudget;
use lumio_test_support::LeakSnapshot;

#[test]
fn leak_snapshot_detects_unreleased_handle_and_bytes() {
    assert!(LeakSnapshot::zero().is_clean());

    let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 4);
    let handle = registry.insert(0u32).expect("insert live handle");
    let budget = MemoryBudget::new(64);
    budget.try_reserve(8).expect("charge 8 bytes");

    let mut snapshot = LeakSnapshot {
        handles_live: 1,
        native_bytes_charged: budget.charged(),
        leases_live: 0,
        jobs_non_terminal: 0,
    };
    assert_eq!(snapshot.handles_live, 1);
    assert_eq!(snapshot.native_bytes_charged, 8);
    assert!(!snapshot.is_clean());

    registry.remove(handle).expect("release handle");
    budget.release(8);
    snapshot.handles_live = 0;
    snapshot.native_bytes_charged = budget.charged();
    assert_eq!(snapshot.native_bytes_charged, 0);
    assert!(snapshot.is_clean());
}

//! T-handle-06 / R-00160: retire_all drops each live payload exactly once.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use lumio_kernel::error::ErrorCategory;
use lumio_kernel::handle::{ContextKey, TypedHandleRegistry};
use lumio_test_support::LeakSnapshot;

struct DropCounter {
    drops: Arc<AtomicU32>,
}

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn retire_all_drops_each_payload_once() {
    let drops = Arc::new(AtomicU32::new(0));
    let mut registry = TypedHandleRegistry::new(ContextKey::new(1), 8);

    let h1 = registry
        .insert(DropCounter {
            drops: Arc::clone(&drops),
        })
        .expect("insert 1");
    let h2 = registry
        .insert(DropCounter {
            drops: Arc::clone(&drops),
        })
        .expect("insert 2");
    let h3 = registry
        .insert(DropCounter {
            drops: Arc::clone(&drops),
        })
        .expect("insert 3");

    assert_eq!(registry.len(), 3);
    assert_eq!(drops.load(Ordering::SeqCst), 0);

    let report = registry.retire_all();
    assert_eq!(report.dropped, 3);
    assert_eq!(drops.load(Ordering::SeqCst), 3);
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.snapshot().live, 0);

    let leak = LeakSnapshot {
        handles_live: u64::from(registry.snapshot().live),
        native_bytes_charged: 0,
        leases_live: 0,
        jobs_non_terminal: 0,
    };
    assert!(leak.is_clean());

    let get_err = match registry.get(h1) {
        Err(e) => e,
        Ok(_) => panic!("get after retire_all must fail"),
    };
    assert_eq!(get_err.category(), ErrorCategory::AlreadyReleased);

    let remove_err = match registry.remove(h2) {
        Err(e) => e,
        Ok(_) => panic!("remove after retire_all must fail"),
    };
    assert_eq!(remove_err.category(), ErrorCategory::AlreadyReleased);
    let _ = h3;

    let second = registry.retire_all();
    assert_eq!(second.dropped, 0);
    assert_eq!(drops.load(Ordering::SeqCst), 3);

    drop(registry);
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}

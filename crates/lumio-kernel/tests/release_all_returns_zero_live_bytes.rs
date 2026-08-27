//! T-memory-06 / R-00161: NativeBufferOwner::release_all returns zero live bytes.

use lumio_kernel::handle::ContextKey;
use lumio_kernel::memory::{
    AllocatorId, MemoryBudget, NativeBufferOwner, NativeBufferReleaseReport,
};
use lumio_test_support::LeakSnapshot;

#[test]
fn release_all_returns_zero_live_bytes() {
    let mut owner = NativeBufferOwner::new(
        ContextKey::new(1),
        4,
        MemoryBudget::new(1000),
        AllocatorId::new(1),
    );

    owner.allocate(8).expect("allocate 8 bytes");
    owner.allocate(16).expect("allocate 16 bytes");
    assert_eq!(owner.charged(), 24);

    let report = owner.release_all();
    assert_eq!(
        report,
        NativeBufferReleaseReport {
            released_buffers: 2,
            released_bytes: 24,
        }
    );
    assert_eq!(owner.charged(), 0);
    assert_eq!(owner.len(), 0);

    let snapshot = LeakSnapshot {
        handles_live: u64::from(owner.len()),
        native_bytes_charged: owner.charged(),
        leases_live: 0,
        jobs_non_terminal: 0,
    };
    assert_eq!(snapshot.native_bytes_charged, 0);
    assert!(snapshot.is_clean());

    let second = owner.release_all();
    assert_eq!(
        second,
        NativeBufferReleaseReport {
            released_buffers: 0,
            released_bytes: 0,
        }
    );
    assert_eq!(owner.charged(), 0);
    assert_eq!(owner.len(), 0);
}

//! T-memory-04 / R-00095: NativeBufferOwner double-release frees budget once.

use lumio_kernel::error::ErrorCategory;
use lumio_kernel::handle::ContextKey;
use lumio_kernel::memory::{AllocatorId, MemoryBudget, NativeBufferOwner};

#[test]
fn double_release_frees_once() {
    let mut owner = NativeBufferOwner::new(
        ContextKey::new(1),
        4,
        MemoryBudget::new(64),
        AllocatorId::new(1),
    );

    let handle = owner.allocate(8).expect("allocate 8 bytes");
    assert_eq!(owner.charged(), 8);

    owner.release(handle).expect("first release");
    assert_eq!(owner.charged(), 0);

    let err = match owner.release(handle) {
        Err(e) => e,
        Ok(()) => panic!("second release must fail"),
    };
    assert_eq!(err.category(), ErrorCategory::AlreadyReleased);
    assert_eq!(owner.charged(), 0);
}

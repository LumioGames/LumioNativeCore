//! T-error-04 / R-00082: error hot path does not heap-allocate.
//!
//! `#[global_allocator]` is test-binary-only (this integration target).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use lumio_kernel::error::{
    ErrorCategory, ErrorDetail, KernelError, MappingBlocked, to_architecture_error_code,
};

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn assert_detail_has_no_string(detail: &ErrorDetail) {
    match detail {
        ErrorDetail::None
        | ErrorDetail::RequiredCapacity {
            required: _,
            provided: _,
        }
        | ErrorDetail::LimitExceeded {
            limit: _,
            requested: _,
        }
        | ErrorDetail::StaticMessage(_) => {}
    }
}

#[test]
fn error_hot_path_does_not_allocate() {
    ALLOC_COUNT.store(0, Ordering::SeqCst);

    let none = KernelError::new(ErrorCategory::Cancelled, ErrorDetail::None);
    let too_small = KernelError::buffer_too_small(64, 8);
    let limit = KernelError::new(
        ErrorCategory::CapacityExceeded,
        ErrorDetail::LimitExceeded {
            limit: 4,
            requested: 8,
        },
    );
    let static_msg = KernelError::new(
        ErrorCategory::InternalInvariant,
        ErrorDetail::StaticMessage("hot-path"),
    );

    let none_category = none.category();
    let none_detail = none.detail();
    let too_small_category = too_small.category();
    let too_small_detail = too_small.detail();
    let limit_category = limit.category();
    let limit_detail = limit.detail();
    let static_category = static_msg.category();
    let static_detail = static_msg.detail();

    let allocs = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(allocs, 0, "error hot path heap-allocated {allocs} time(s)");

    assert_eq!(none_category, ErrorCategory::Cancelled);
    assert_eq!(too_small_category, ErrorCategory::BufferTooSmall);
    assert_eq!(limit_category, ErrorCategory::CapacityExceeded);
    assert_eq!(static_category, ErrorCategory::InternalInvariant);

    assert_detail_has_no_string(none_detail);
    assert_detail_has_no_string(too_small_detail);
    assert_detail_has_no_string(limit_detail);
    assert_detail_has_no_string(static_detail);

    match none_detail {
        ErrorDetail::None => {}
        other => panic!("unexpected detail: {other:?}"),
    }
    match too_small_detail {
        ErrorDetail::RequiredCapacity { required, provided } => {
            assert_eq!(*required, 64);
            assert_eq!(*provided, 8);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    match limit_detail {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 4);
            assert_eq!(*requested, 8);
        }
        other => panic!("unexpected detail: {other:?}"),
    }
    match static_detail {
        ErrorDetail::StaticMessage(msg) => assert_eq!(*msg, "hot-path"),
        other => panic!("unexpected detail: {other:?}"),
    }

    assert_eq!(to_architecture_error_code(&none), Err(MappingBlocked));
    assert_eq!(to_architecture_error_code(&too_small), Err(MappingBlocked));
    assert_eq!(to_architecture_error_code(&limit), Err(MappingBlocked));
    assert_eq!(to_architecture_error_code(&static_msg), Err(MappingBlocked));
}

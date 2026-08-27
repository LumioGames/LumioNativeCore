//! T-job-05 / R-00147: a full bounded queue fails immediately, never blocks.

use lumio_job::BoundedJobQueue;
use lumio_kernel::error::{ErrorCategory, ErrorDetail};

#[test]
fn queue_full_returns_immediately() {
    let q = BoundedJobQueue::with_capacity(1);
    assert_eq!(q.cap(), 1);

    assert!(q.try_push(1u32).is_ok());

    let err = q
        .try_push(2u32)
        .expect_err("full queue must return CapacityExceeded immediately");
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
    match err.detail() {
        ErrorDetail::LimitExceeded { limit, requested } => {
            assert_eq!(*limit, 1);
            assert_eq!(*requested, 2);
        }
        other => panic!("unexpected detail: {other:?}"),
    }

    assert_eq!(q.try_pop(), Some(1u32));
    assert!(q.try_push(3u32).is_ok());
}

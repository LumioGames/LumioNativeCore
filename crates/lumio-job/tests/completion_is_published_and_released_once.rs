//! T-job-07 / R-00156: a completion is published once and released once.

use lumio_job::{CompletionBatch, JobCompletion, JobId, JobState};
use lumio_kernel::error::ErrorCategory;

#[test]
fn completion_is_published_and_released_once() {
    let batch = CompletionBatch::with_capacity(1);
    let id = JobId::from_raw(1);
    let completion = JobCompletion {
        id,
        state: JobState::Succeeded,
    };

    batch
        .publish(completion)
        .expect("first publish must succeed");

    let dup = batch
        .publish(completion)
        .expect_err("second publish of the same JobId must be rejected");
    assert_eq!(dup.category(), ErrorCategory::InvalidArgument);

    let overflow = batch
        .publish(JobCompletion {
            id: JobId::from_raw(2),
            state: JobState::Failed,
        })
        .expect_err("publish beyond capacity must fail immediately");
    assert_eq!(overflow.category(), ErrorCategory::CapacityExceeded);

    let mut empty: [JobCompletion; 0] = [];
    assert_eq!(
        batch.drain(&mut empty).expect("empty drain"),
        0,
        "empty output must not consume queued completions"
    );

    let mut out = [JobCompletion {
        id: JobId::from_raw(0),
        state: JobState::Queued,
    }; 1];
    let n = batch.drain(&mut out).expect("first drain");
    assert_eq!(n, 1);
    assert_eq!(out[0].id, id);
    assert_eq!(out[0].state, JobState::Succeeded);

    let n2 = batch.drain(&mut out).expect("second drain");
    assert_eq!(n2, 0);

    batch.release(id).expect("first release must succeed");
    let released = batch.release(id).expect_err("second release must fail");
    assert_eq!(released.category(), ErrorCategory::AlreadyReleased);

    let republish = batch
        .publish(completion)
        .expect_err("released JobId must stay consumed");
    assert_eq!(republish.category(), ErrorCategory::InvalidArgument);
}

//! T-job-06 / R-00148: worker execute must not hold the scheduler lock.

use std::sync::Arc;

use lumio_job::{
    JobRequest, JobState, JobSystem, JobSystemConfig, OperationId, OperationRegistry, TypedKernel,
};
use lumio_kernel::capability::ConfiguredLimits;
use lumio_kernel::context::{ContextConfig, KernelContext};
use lumio_kernel::error::ErrorCategory;
use lumio_platform::{Deadline, Ticks};
use lumio_test_support::FakeClock;

struct DummyKernel {
    id: OperationId,
}

impl TypedKernel for DummyKernel {
    fn operation_id(&self) -> OperationId {
        self.id
    }
}

fn test_config() -> ContextConfig {
    ContextConfig {
        limits: ConfiguredLimits {
            max_handles: 4,
            max_native_bytes: 64,
            max_jobs_queued: 4,
            max_jobs_running: 1,
            max_completion_items: 1,
        },
        quiesce_deadline: Deadline::NONE,
    }
}

#[test]
fn worker_never_executes_under_scheduler_lock() {
    let context = KernelContext::create_for_test(test_config());
    let mut registry = OperationRegistry::new();
    let op = OperationId::test_only(1);
    registry
        .register(Arc::new(DummyKernel { id: op }))
        .expect("register dummy kernel");
    let system = JobSystem::create(
        context,
        JobSystemConfig {
            queue_capacity: 2,
            worker_count: 0,
        },
        Arc::new(registry),
        Arc::new(FakeClock::new(Ticks::ZERO)),
    )
    .expect("create job system");

    let handle = system
        .submit(JobRequest {
            operation: op,
            deadline: Deadline::NONE,
        })
        .expect("submit");

    assert!(system.pump_one(), "queued job must be pumped");
    assert!(
        !system.scheduler_lock_held(),
        "execute region must not observe the scheduler lock as held"
    );

    let snap = system.poll(handle).expect("poll");
    assert_eq!(snap.id, handle.id());
    assert_eq!(snap.state, JobState::Succeeded);

    let _second = system
        .submit(JobRequest {
            operation: op,
            deadline: Deadline::NONE,
        })
        .expect("queue still has a free slot");
    let _third = system
        .submit(JobRequest {
            operation: op,
            deadline: Deadline::NONE,
        })
        .expect("fill remaining capacity");
    let err = system
        .submit(JobRequest {
            operation: op,
            deadline: Deadline::NONE,
        })
        .expect_err("full queue must return CapacityExceeded immediately");
    assert_eq!(err.category(), ErrorCategory::CapacityExceeded);
}

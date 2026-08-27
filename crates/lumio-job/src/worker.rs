//! Scheduler + deterministic worker pump.
//!
//! The scheduler mutex is held only for `try_pop`. Kernel lookup and state CAS
//! run after that mutex is released.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use lumio_kernel::context::KernelContext;
use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};
use lumio_platform::{Deadline, MonotonicClock};

use crate::id::{JobId, OperationId};
use crate::operation::OperationRegistry;
use crate::queue::BoundedJobQueue;
use crate::state::{JobState, JobStateMachine};

const ORDER: Ordering = Ordering::SeqCst;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobSystemConfig {
    pub queue_capacity: usize,
    pub worker_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobRequest {
    pub operation: OperationId,
    pub deadline: Deadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct JobHandle {
    id: JobId,
}

impl JobHandle {
    pub fn id(self) -> JobId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobSnapshot {
    pub id: JobId,
    pub state: JobState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelOutcome {
    Cancelled { previous: JobState },
    AlreadyTerminal(JobState),
}

struct JobRecord {
    operation: OperationId,
    deadline: Deadline,
    state: JobStateMachine,
}

pub struct JobSystem {
    context: Arc<KernelContext>,
    registry: Arc<OperationRegistry>,
    clock: Arc<dyn MonotonicClock>,
    queue: BoundedJobQueue<JobId>,
    jobs: Mutex<HashMap<JobId, Arc<JobRecord>>>,
    next_id: AtomicU64,
    scheduler: Mutex<()>,
    lock_held: AtomicBool,
    execute_saw_lock: AtomicBool,
}

impl JobSystem {
    pub fn create(
        context: Arc<KernelContext>,
        config: JobSystemConfig,
        registry: Arc<OperationRegistry>,
        clock: Arc<dyn MonotonicClock>,
    ) -> KernelResult<Arc<Self>> {
        if config.queue_capacity == 0 {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        Ok(Arc::new(Self {
            context,
            registry,
            clock,
            queue: BoundedJobQueue::with_capacity(config.queue_capacity),
            jobs: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            scheduler: Mutex::new(()),
            lock_held: AtomicBool::new(false),
            execute_saw_lock: AtomicBool::new(false),
        }))
    }

    pub fn submit(&self, request: JobRequest) -> KernelResult<JobHandle> {
        self.context.ensure_accepting_work()?;
        let id = JobId::from_raw(self.next_id.fetch_add(1, ORDER));
        let record = Arc::new(JobRecord {
            operation: request.operation,
            deadline: request.deadline,
            state: JobStateMachine::queued(),
        });
        {
            let mut jobs = self.jobs.lock().unwrap_or_else(|p| p.into_inner());
            jobs.insert(id, Arc::clone(&record));
        }
        match self.queue.try_push(id) {
            Ok(()) => Ok(JobHandle { id }),
            Err(err) => {
                let mut jobs = self.jobs.lock().unwrap_or_else(|p| p.into_inner());
                jobs.remove(&id);
                Err(err)
            }
        }
    }

    pub fn poll(&self, handle: JobHandle) -> KernelResult<JobSnapshot> {
        let jobs = self.jobs.lock().unwrap_or_else(|p| p.into_inner());
        let record = jobs
            .get(&handle.id)
            .ok_or_else(|| KernelError::new(ErrorCategory::InvalidHandle, ErrorDetail::None))?;
        Ok(JobSnapshot {
            id: handle.id,
            state: record.state.snapshot(),
        })
    }

    pub fn cancel(&self, handle: JobHandle) -> KernelResult<CancelOutcome> {
        let record = {
            let jobs = self.jobs.lock().unwrap_or_else(|p| p.into_inner());
            jobs.get(&handle.id)
                .cloned()
                .ok_or_else(|| KernelError::new(ErrorCategory::InvalidHandle, ErrorDetail::None))?
        };
        match record.state.cas_cancel() {
            Ok(previous) => Ok(CancelOutcome::Cancelled { previous }),
            Err(terminal) => Ok(CancelOutcome::AlreadyTerminal(terminal)),
        }
    }

    /// Dequeue one job under the scheduler lock, then execute with it released.
    /// Returns whether a job was dequeued.
    pub fn pump_one(&self) -> bool {
        let id = {
            let _scheduler = self.scheduler.lock().unwrap_or_else(|p| p.into_inner());
            self.lock_held.store(true, ORDER);
            let id = self.queue.try_pop();
            self.lock_held.store(false, ORDER);
            id
        };
        let Some(id) = id else {
            return false;
        };
        self.execute_one(id);
        true
    }

    /// True iff the last execute region observed the scheduler lock as held.
    pub fn scheduler_lock_held(&self) -> bool {
        self.execute_saw_lock.load(ORDER)
    }

    fn execute_one(&self, id: JobId) {
        self.execute_saw_lock
            .store(self.lock_held.load(ORDER), ORDER);

        let Some(record) = ({
            let jobs = self.jobs.lock().unwrap_or_else(|p| p.into_inner());
            jobs.get(&id).cloned()
        }) else {
            return;
        };

        if record.deadline.is_expired(self.clock.now()) {
            let _ = record.state.cas_cancel();
            return;
        }

        if record.state.cas_start().is_err() {
            return;
        }

        let outcome = if self.registry.get(record.operation).is_some() {
            JobState::Succeeded
        } else {
            JobState::Failed
        };
        let _ = record.state.cas_complete(outcome);
    }
}

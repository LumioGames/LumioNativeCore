//! One-shot completion batch: publish, drain, and release each JobId at most once.

use std::collections::HashMap;
use std::sync::Mutex;

use lumio_kernel::error::{ErrorCategory, ErrorDetail, KernelError, KernelResult};

use crate::id::JobId;
use crate::queue::BoundedJobQueue;
use crate::state::JobState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobCompletion {
    pub id: JobId,
    pub state: JobState,
}

enum Lease {
    Published,
    Released,
}

pub struct CompletionBatch {
    queue: BoundedJobQueue<JobCompletion>,
    leases: Mutex<HashMap<JobId, Lease>>,
}

impl CompletionBatch {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            queue: BoundedJobQueue::with_capacity(cap),
            leases: Mutex::new(HashMap::new()),
        }
    }

    pub fn publish(&self, c: JobCompletion) -> KernelResult<()> {
        let mut leases = self.leases.lock().unwrap_or_else(|p| p.into_inner());
        if leases.contains_key(&c.id) {
            return Err(KernelError::new(
                ErrorCategory::InvalidArgument,
                ErrorDetail::None,
            ));
        }
        let id = c.id;
        self.queue.try_push(c)?;
        leases.insert(id, Lease::Published);
        Ok(())
    }

    pub fn drain(&self, out: &mut [JobCompletion]) -> KernelResult<usize> {
        let mut n = 0;
        for slot in out.iter_mut() {
            match self.queue.try_pop() {
                Some(c) => {
                    *slot = c;
                    n += 1;
                }
                None => break,
            }
        }
        Ok(n)
    }

    pub fn release(&self, id: JobId) -> KernelResult<()> {
        let mut leases = self.leases.lock().unwrap_or_else(|p| p.into_inner());
        match leases.get_mut(&id) {
            Some(lease @ Lease::Published) => {
                *lease = Lease::Released;
                Ok(())
            }
            Some(Lease::Released) => Err(KernelError::new(
                ErrorCategory::AlreadyReleased,
                ErrorDetail::None,
            )),
            None => Err(KernelError::new(
                ErrorCategory::InvalidHandle,
                ErrorDetail::None,
            )),
        }
    }
}
